//! The MCP server handler — the three weather tools (tool-specs §2–§4).
//!
//! Exposes exactly `get_forecast`, `get_historical`, `compare_period`. The request structs are
//! the tool contract (their `JsonSchema` surfaces in `tools/list`); the result structs pin the
//! output envelope + payload shapes the `insta` snapshots assert.
//!
//! The server is **independent of the transport** (the transport seam): `serve(...)` in `main.rs`
//! decides stdio vs. HTTP. It holds the [`WeatherData`] client behind `Arc<dyn …>` so the binary
//! can pick the fixture-backed or real-HTTP impl at runtime.
//!
//! The handlers run the pure pipeline behind the [`WeatherData`] seam: resolve the location
//! (geocode or coordinates), fetch forecast/archive, run the date guards + ERA5-lag clamp +
//! `compare` aggregation, and assemble the §1.6 envelope. A [`WeatherError`]
//! (§1.5) surfaces as a success-typed `CallToolResult` with `is_error: true` (so the model can
//! read + recover), never a protocol-level error.

use std::sync::Arc;

use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Implementation, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler,
};
use serde::{Deserialize, Serialize};

use crate::compare::{self, ComparePayload, CompareSpec};
use crate::dates::{clamp_end_to_archive, validate_date_range};
use crate::location::{
    location_from_coordinates, parse_location_input, resolve_geocoded, LocationInput,
};
use crate::openmeteo::{
    archive::ArchiveDaily,
    forecast::{Current, DailyForecast},
    ArchiveQuery, ForecastQuery, WeatherData,
};
use crate::types::{default_variables, Envelope, Location, Notes, Units, Variable, WeatherError};

pub const SERVER_NAME: &str = "weather-mcp";
pub const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const SERVER_DESCRIPTION: &str =
    "MCP server wrapping the Open-Meteo API (forecast + historical trends).";

// ---------------------------------------------------------------------------------------------
// Request contracts (§1.1–§1.4, §2–§4). These derive `JsonSchema` so the schema is published in
// `tools/list`; the snapshot pins it.
// ---------------------------------------------------------------------------------------------

fn default_forecast_days() -> u8 {
    7
}
fn default_baseline_start_year() -> i32 {
    1991
}
fn default_baseline_end_year() -> i32 {
    2020
}

/// `get_forecast` request (§2). Exactly one of `location` vs `latitude`+`longitude` (§1.1).
#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
pub struct ForecastRequest {
    #[serde(default)]
    pub location: Option<String>,
    #[serde(default)]
    pub latitude: Option<f64>,
    #[serde(default)]
    pub longitude: Option<f64>,
    /// 1–16; Open-Meteo max is 16.
    #[serde(default = "default_forecast_days")]
    pub forecast_days: u8,
    #[serde(default)]
    pub units: Units,
}

/// `get_historical` request (§3).
#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
pub struct HistoricalRequest {
    #[serde(default)]
    pub location: Option<String>,
    #[serde(default)]
    pub latitude: Option<f64>,
    #[serde(default)]
    pub longitude: Option<f64>,
    /// `YYYY-MM-DD`, ≥ 1940-01-01.
    pub start_date: String,
    /// `YYYY-MM-DD`, clamped per the ERA5 lag (§1.7).
    pub end_date: String,
    #[serde(default = "default_variables")]
    pub variables: Vec<Variable>,
    #[serde(default)]
    pub units: Units,
}

/// A `YYYY-MM-DD` start/end window of interest (§4.3).
#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
pub struct Period {
    pub start: String,
    pub end: String,
}

/// `compare_period` request (§4.3).
#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
pub struct CompareRequest {
    #[serde(default)]
    pub location: Option<String>,
    #[serde(default)]
    pub latitude: Option<f64>,
    #[serde(default)]
    pub longitude: Option<f64>,
    pub period: Period,
    #[serde(default = "default_variables")]
    pub variables: Vec<Variable>,
    /// ≥ 1940 (§4.1).
    #[serde(default = "default_baseline_start_year")]
    pub baseline_start_year: i32,
    /// ≥ 1940 (§4.1).
    #[serde(default = "default_baseline_end_year")]
    pub baseline_end_year: i32,
    /// Adds the day-by-day series for the time-series view (§4.5).
    #[serde(default)]
    pub include_series: bool,
    #[serde(default)]
    pub units: Units,
}

// ---------------------------------------------------------------------------------------------
// Result contracts (§1.6 envelope + per-tool payload). Output-only; the snapshots pin these.
// ---------------------------------------------------------------------------------------------

/// `get_forecast` success result: shared envelope + current + daily (§2).
#[derive(Debug, Clone, Serialize)]
pub struct ForecastResult {
    #[serde(flatten)]
    pub envelope: Envelope,
    pub current: Current,
    pub daily: DailyForecast,
}

/// The explicit window echoed by `get_historical` (§3).
#[derive(Debug, Clone, Serialize)]
pub struct RangeInfo {
    pub start: String,
    pub end: String,
}

/// `get_historical` success result: envelope + range + the curated daily columns (§3).
#[derive(Debug, Clone, Serialize)]
pub struct HistoricalResult {
    #[serde(flatten)]
    pub envelope: Envelope,
    pub range: RangeInfo,
    pub daily: ArchiveDaily,
}

/// `compare_period` success result: envelope + the comparison payload (§4.4).
#[derive(Debug, Clone, Serialize)]
pub struct CompareResult {
    #[serde(flatten)]
    pub envelope: Envelope,
    #[serde(flatten)]
    pub payload: ComparePayload,
}

// ---------------------------------------------------------------------------------------------
// The server
// ---------------------------------------------------------------------------------------------

/// The weather MCP server. Holds the tool router and the Open-Meteo data seam.
#[derive(Clone)]
pub struct WeatherServer {
    // Read by the `#[tool_handler]`-generated `call_tool`/`list_tools`; the macro hides the use
    // from dead-code analysis, hence the allow (as in Phase 0).
    #[allow(dead_code)]
    tool_router: ToolRouter<Self>,
    // The Open-Meteo data source (fixture-backed or real HTTP). The handlers call it;
    // the binary selects the impl at startup.
    client: Arc<dyn WeatherData>,
}

#[tool_router]
impl WeatherServer {
    /// Build a server over the given data source.
    pub fn new(client: Arc<dyn WeatherData>) -> Self {
        Self {
            tool_router: Self::tool_router(),
            client,
        }
    }

    /// Current conditions + an N-day daily forecast for a location (§2).
    #[tool(
        description = "Current conditions and an N-day daily forecast for a location (by name or \
                       coordinates). Returns current weather plus a chart-friendly daily series."
    )]
    async fn get_forecast(
        &self,
        Parameters(req): Parameters<ForecastRequest>,
    ) -> Result<CallToolResult, McpError> {
        match self.run_forecast(req).await {
            Ok(value) => Ok(CallToolResult::structured(value)),
            Err(e) => Ok(error_result(e)),
        }
    }

    /// The daily ERA5 record for an explicit historical window (§3).
    #[tool(
        description = "The daily historical weather record (ERA5) for an explicit date window: \
                       temperature, precipitation, snowfall, and/or wind for the requested variables."
    )]
    async fn get_historical(
        &self,
        Parameters(req): Parameters<HistoricalRequest>,
    ) -> Result<CallToolResult, McpError> {
        match self.run_historical(req).await {
            Ok(value) => Ok(CallToolResult::structured(value)),
            Err(e) => Ok(error_result(e)),
        }
    }

    /// Aggregate a period and compare it against a climate baseline (§4) — the differentiator.
    #[tool(
        description = "Compare a period of interest against a climate baseline using calendar-window \
                       matching: returns the period aggregate, the per-year baseline distribution, and \
                       anomaly scores (absolute, percent, z-score, percentile, rank)."
    )]
    async fn compare_period(
        &self,
        Parameters(req): Parameters<CompareRequest>,
    ) -> Result<CallToolResult, McpError> {
        match self.run_compare(req).await {
            Ok(value) => Ok(CallToolResult::structured(value)),
            Err(e) => Ok(error_result(e)),
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Handler pipelines. Each returns the serialized success payload or a structured [`WeatherError`]
// (§1.5); the `#[tool]` wrappers map a success to `structured` and an error to `error_result`
// (a success-typed protocol result carrying `is_error: true`), never a protocol-level `McpError`.
// ---------------------------------------------------------------------------------------------

/// Either a place name (geocode) or directly-supplied coordinates, resolved into the seed of the
/// envelope's [`Location`]: for the name path we pick the top hit + ambiguity notes now; for the
/// coordinate path the `Location` is deferred until the weather response supplies tz/elevation.
enum ResolvedLocation {
    /// Geocoded: a partially-filled location (tz/elevation overridden post-fetch) + ambiguity notes.
    Geocoded { location: Location, notes: Notes },
    /// Coordinates: build the location after the fetch via [`location_from_coordinates`].
    Coordinates { latitude: f64, longitude: f64 },
}

impl ResolvedLocation {
    /// The latitude/longitude to query upstream with (geocode hit's, or the supplied coords).
    fn query_coords(&self) -> (f64, f64) {
        match self {
            ResolvedLocation::Geocoded { location, .. } => (location.latitude, location.longitude),
            ResolvedLocation::Coordinates {
                latitude,
                longitude,
            } => (*latitude, *longitude),
        }
    }
}

impl WeatherServer {
    /// Resolve the location params into a [`ResolvedLocation`], geocoding for the name path.
    async fn resolve_location(
        &self,
        location: Option<String>,
        latitude: Option<f64>,
        longitude: Option<f64>,
    ) -> Result<ResolvedLocation, WeatherError> {
        match parse_location_input(location, latitude, longitude)? {
            LocationInput::Name(name) => {
                let hits = self.client.geocode(&name, 10).await?;
                let (location, notes) = resolve_geocoded(&hits, &name)?;
                Ok(ResolvedLocation::Geocoded { location, notes })
            }
            LocationInput::Coordinates {
                latitude,
                longitude,
            } => Ok(ResolvedLocation::Coordinates {
                latitude,
                longitude,
            }),
        }
    }

    async fn run_forecast(&self, req: ForecastRequest) -> Result<serde_json::Value, WeatherError> {
        let resolved = self
            .resolve_location(req.location, req.latitude, req.longitude)
            .await?;
        let (latitude, longitude) = resolved.query_coords();

        let query = ForecastQuery {
            latitude,
            longitude,
            forecast_days: req.forecast_days,
            units: req.units,
        };
        let payload = self.client.forecast(&query).await?;

        let (location, notes) =
            finalize_location(resolved, payload.elevation, payload.timezone.clone());

        let result = ForecastResult {
            envelope: Envelope {
                location,
                units: req.units.labels(),
                notes,
            },
            current: payload.current,
            daily: payload.daily,
        };
        to_json_value(&result)
    }

    async fn run_historical(
        &self,
        req: HistoricalRequest,
    ) -> Result<serde_json::Value, WeatherError> {
        validate_date_range(&req.start_date, &req.end_date, &today())?;

        let resolved = self
            .resolve_location(req.location, req.latitude, req.longitude)
            .await?;
        let (latitude, longitude) = resolved.query_coords();

        let columns = column_union(&req.variables);
        let query = ArchiveQuery {
            latitude,
            longitude,
            start_date: req.start_date.clone(),
            end_date: req.end_date.clone(),
            columns: columns.clone(),
            units: req.units,
        };
        let data = self.client.archive(&query).await?;

        // Clamp the requested end down to the last archived day (ERA5 ~5-day lag, §1.7).
        let (effective_end, clamp_note) = match data.daily.time.last() {
            Some(last) if last.as_str() < req.end_date.as_str() => {
                clamp_end_to_archive(&req.end_date, last)
            }
            _ => (req.end_date.clone(), None),
        };

        let (location, mut notes) =
            finalize_location(resolved, data.elevation, data.timezone.clone());
        if let Some(note) = clamp_note {
            notes.push(note);
        }

        // Project to only the requested columns (drop any extras the fixture carries, in order).
        let projected = project_columns(data.daily, &columns);

        let result = HistoricalResult {
            envelope: Envelope {
                location,
                units: req.units.labels(),
                notes,
            },
            range: RangeInfo {
                start: req.start_date,
                end: effective_end,
            },
            daily: projected,
        };
        to_json_value(&result)
    }

    async fn run_compare(&self, req: CompareRequest) -> Result<serde_json::Value, WeatherError> {
        let resolved = self
            .resolve_location(req.location, req.latitude, req.longitude)
            .await?;
        let (latitude, longitude) = resolved.query_coords();

        let columns = column_union(&req.variables);

        // Baseline window spans whole calendar years; the period is the requested window.
        let baseline_start = format!("{}-01-01", req.baseline_start_year);
        let baseline_end = format!("{}-12-31", req.baseline_end_year);

        let archive_query = |start: String, end: String| ArchiveQuery {
            latitude,
            longitude,
            start_date: start,
            end_date: end,
            columns: columns.clone(),
            units: req.units,
        };

        // The period fetch — clamp its end down to the last archived day (§1.7), not a future error.
        let period_data = self
            .client
            .archive(&archive_query(
                req.period.start.clone(),
                req.period.end.clone(),
            ))
            .await?;
        let (effective_period_end, clamp_note) = match period_data.daily.time.last() {
            Some(last) if last.as_str() < req.period.end.as_str() => {
                clamp_end_to_archive(&req.period.end, last)
            }
            _ => (req.period.end.clone(), None),
        };

        let baseline_data = self
            .client
            .archive(&archive_query(baseline_start, baseline_end))
            .await?;

        let spec = CompareSpec {
            period_start: req.period.start,
            period_end: effective_period_end,
            variables: req.variables,
            baseline_start_year: req.baseline_start_year,
            baseline_end_year: req.baseline_end_year,
            include_series: req.include_series,
            units: req.units,
        };
        let out = compare::compare(&baseline_data, &period_data, &spec)?;

        // tz/elevation come from the archive response (period and baseline agree on the location).
        let (location, mut notes) = finalize_location(
            resolved,
            period_data.elevation,
            period_data.timezone.clone(),
        );
        // Merge order: location notes, then any clamp note, then the aggregation notes (§4.2).
        if let Some(note) = clamp_note {
            notes.push(note);
        }
        notes.extend(out.notes);

        let result = CompareResult {
            envelope: Envelope {
                location,
                units: req.units.labels(),
                notes,
            },
            payload: out.payload,
        };
        to_json_value(&result)
    }
}

/// Build a structured error result (§1.5): a success-typed protocol [`CallToolResult`] carrying
/// `is_error: true` and the `{ "error": { … } }` body, so the model can read and recover.
fn error_result(e: WeatherError) -> CallToolResult {
    CallToolResult::structured_error(e.to_json())
}

/// Serialize a success result to a JSON value. The result structs are plain data with no
/// fallible custom serializers, so this never errors in practice; if it somehow did, surface it
/// as an `upstream_error` (§1.5) rather than panicking the handler.
fn to_json_value<T: Serialize>(result: &T) -> Result<serde_json::Value, WeatherError> {
    serde_json::to_value(result).map_err(|e| {
        WeatherError::new(
            crate::types::ErrorCode::UpstreamError,
            format!("failed to serialize result: {e}"),
        )
    })
}

/// Finalize the envelope's [`Location`], overriding timezone + elevation from the weather/archive
/// response (the authoritative resolved values, §1.1): geocoded hits keep their place fields but
/// take tz/elevation from the response; coordinate inputs are built fresh here.
fn finalize_location(
    resolved: ResolvedLocation,
    response_elevation: f64,
    response_timezone: String,
) -> (Location, Notes) {
    match resolved {
        ResolvedLocation::Geocoded {
            mut location,
            notes,
        } => {
            location.timezone = response_timezone;
            location.elevation = response_elevation;
            (location, notes)
        }
        ResolvedLocation::Coordinates {
            latitude,
            longitude,
        } => {
            let location = location_from_coordinates(
                latitude,
                longitude,
                response_elevation,
                response_timezone,
            );
            (location, Notes::new())
        }
    }
}

/// The in-order union of every variable's archive columns: first-seen order, de-duplicated (§1.4).
fn column_union(variables: &[Variable]) -> Vec<String> {
    let mut columns: Vec<String> = Vec::new();
    for var in variables {
        for col in var.archive_columns() {
            if !columns.iter().any(|c| c == col) {
                columns.push((*col).to_string());
            }
        }
    }
    columns
}

/// Keep only the requested columns from a parsed daily block (drop any extras the fixture carries),
/// preserving `time` alignment. Missing columns are simply skipped.
fn project_columns(daily: ArchiveDaily, columns: &[String]) -> ArchiveDaily {
    let ArchiveDaily {
        time,
        columns: mut src,
    } = daily;
    let mut kept = std::collections::BTreeMap::new();
    for col in columns {
        if let Some(values) = src.remove(col) {
            kept.insert(col.clone(), values);
        }
    }
    ArchiveDaily {
        time,
        columns: kept,
    }
}

/// Today as `YYYY-MM-DD` in UTC, dependency-free from the system clock (Howard Hinnant's
/// civil-from-days algorithm). Used only for the future-end date guard (§1.5); the conformance
/// dates are historical so its exact value never affects a snapshot.
fn today() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = (secs / 86_400) as i64; // days since 1970-01-01 (UTC)

    // civil_from_days (Hinnant): days since the epoch → (year, month, day).
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    let year = if m <= 2 { y + 1 } else { y };

    format!("{year:04}-{m:02}-{d:02}")
}

#[tool_handler]
impl ServerHandler for WeatherServer {
    fn get_info(&self) -> ServerInfo {
        // `ServerInfo` / `Implementation` are `#[non_exhaustive]` — start from `default()`.
        let mut server_info = Implementation::default();
        server_info.name = SERVER_NAME.to_string();
        server_info.version = SERVER_VERSION.to_string();

        let mut info = ServerInfo::default();
        info.capabilities = ServerCapabilities::builder().enable_tools().build();
        info.server_info = server_info;
        info.instructions = Some(SERVER_DESCRIPTION.to_string());
        info
    }
}
