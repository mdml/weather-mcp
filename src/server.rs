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
//! **Phase 2 status:** the handlers are stubbed — they return a clean protocol error so the
//! conformance child process never panics. `tools/list` (names + schemas) is fully live now; the
//! `tools/call` paths go green in Phase 3 once the pure pipeline behind the seam is filled in.
//! Each handler documents the exact Phase 3 wiring it will grow.

use std::sync::Arc;

use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Implementation, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler,
};
use serde::{Deserialize, Serialize};

use crate::compare::ComparePayload;
use crate::openmeteo::{
    archive::ArchiveDaily,
    forecast::{Current, DailyForecast},
    WeatherData,
};
use crate::types::{default_variables, Envelope, Units, Variable};

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
    // The Open-Meteo data source (fixture-backed or real HTTP). Unused until the Phase 3 handlers
    // call it; kept here now so the seam is wired and the binary selects the impl at startup.
    #[allow(dead_code)]
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
        Parameters(_req): Parameters<ForecastRequest>,
    ) -> Result<CallToolResult, McpError> {
        // Phase 3 wiring:
        //   let input = location::parse_location_input(req.location, req.latitude, req.longitude)?;
        //   let (location, mut notes) = resolve(input via self.client.geocode | coordinates);
        //   let payload = self.client.forecast(&ForecastQuery { .. }).await?;  // fill tz/elev
        //   let result = ForecastResult { envelope: Envelope { location, units, notes }, .. };
        //   Ok(CallToolResult::structured(serde_json::to_value(result)?))
        Err(McpError::internal_error(
            "get_forecast not implemented yet (Phase 3)",
            None,
        ))
    }

    /// The daily ERA5 record for an explicit historical window (§3).
    #[tool(
        description = "The daily historical weather record (ERA5) for an explicit date window: \
                       temperature, precipitation, snowfall, and/or wind for the requested variables."
    )]
    async fn get_historical(
        &self,
        Parameters(_req): Parameters<HistoricalRequest>,
    ) -> Result<CallToolResult, McpError> {
        // Phase 3 wiring:
        //   validate dates (§1.5 invalid_date_range), resolve location, clamp end (§1.7 -> notes),
        //   let data = self.client.archive(&ArchiveQuery { columns: union of variables, .. }).await?;
        //   Ok(CallToolResult::structured(HistoricalResult { .. }))
        Err(McpError::internal_error(
            "get_historical not implemented yet (Phase 3)",
            None,
        ))
    }

    /// Aggregate a period and compare it against a climate baseline (§4) — the differentiator.
    #[tool(
        description = "Compare a period of interest against a climate baseline using calendar-window \
                       matching: returns the period aggregate, the per-year baseline distribution, and \
                       anomaly scores (absolute, percent, z-score, percentile, rank)."
    )]
    async fn compare_period(
        &self,
        Parameters(_req): Parameters<CompareRequest>,
    ) -> Result<CallToolResult, McpError> {
        // Phase 3 wiring:
        //   resolve location; build baseline + period ArchiveQuery (§4.6, ~2 fetches);
        //   let out = compare::compare(&baseline, &period, &spec)?;  // out.notes -> envelope.notes
        //   Ok(CallToolResult::structured(CompareResult { envelope, payload: out.payload }))
        Err(McpError::internal_error(
            "compare_period not implemented yet (Phase 3)",
            None,
        ))
    }
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
