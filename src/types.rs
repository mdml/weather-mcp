//! Shared types — the contract vocabulary used across every tool.
//!
//! These mirror [tool-specs](../../docs/design/tool-specs.md) §1 (shared conventions): the
//! `units` enum, the curated `variables` enum + its aggregation/column mapping, the shared
//! output envelope (`location` / `units` / `notes`), and the structured error model (§1.5).
//!
//! Everything here is pure data + small total functions — no I/O. The tool *logic* (parse,
//! aggregate, resolve, assemble) lives in the sibling modules; this vocabulary is the stable
//! surface they and the tests both compile against.

use serde::{Deserialize, Serialize};

/// Human-readable `notes` (clamps, ambiguity, dropped days) — non-fatal per §1.5.
pub type Notes = Vec<String>;

// ---------------------------------------------------------------------------------------------
// Units (§1.2)
// ---------------------------------------------------------------------------------------------

/// The single `units` knob (§1.2). Maps to Open-Meteo's three unit params and to the labels
/// echoed in every output envelope.
///
/// `#[schemars(inline)]` forces this enum's schema to be emitted at every use site rather than
/// behind a JSON-Schema reference into a shared definitions block. Many MCP clients / LLM
/// tool-callers don't follow those references, so the `metric`/`imperial` choices must appear
/// inline at the param — otherwise callers have to hand-quote the value blind.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, schemars::JsonSchema,
)]
#[schemars(inline)]
#[serde(rename_all = "lowercase")]
pub enum Units {
    /// °C · mm · km/h.
    #[default]
    Metric,
    /// °F · inch · mph.
    Imperial,
}

/// The resolved unit labels echoed in the output envelope so the model never guesses (§1.2/§1.6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct UnitLabels {
    pub temperature: String,
    pub precipitation: String,
    pub wind_speed: String,
}

impl Units {
    /// The display labels for this unit system (echoed in the envelope).
    pub fn labels(self) -> UnitLabels {
        match self {
            Units::Metric => UnitLabels {
                temperature: "°C".to_string(),
                precipitation: "mm".to_string(),
                wind_speed: "km/h".to_string(),
            },
            Units::Imperial => UnitLabels {
                temperature: "°F".to_string(),
                precipitation: "inch".to_string(),
                wind_speed: "mph".to_string(),
            },
        }
    }

    /// Open-Meteo `temperature_unit` wire value.
    pub fn temperature_param(self) -> &'static str {
        match self {
            Units::Metric => "celsius",
            Units::Imperial => "fahrenheit",
        }
    }

    /// Open-Meteo `precipitation_unit` wire value.
    pub fn precipitation_param(self) -> &'static str {
        match self {
            Units::Metric => "mm",
            Units::Imperial => "inch",
        }
    }

    /// Open-Meteo `wind_speed_unit` wire value.
    pub fn wind_speed_param(self) -> &'static str {
        match self {
            Units::Metric => "kmh",
            Units::Imperial => "mph",
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Curated variable set (§1.4)
// ---------------------------------------------------------------------------------------------

/// How a daily series is collapsed to one period scalar (§1.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Aggregation {
    /// Period total (precipitation, snowfall).
    Sum,
    /// Period mean of the daily values (temperature, wind).
    Mean,
}

/// The fixed, snapshot-testable variable enum for the historical/comparison path (§1.4).
/// `get_forecast` is intentionally *not* limited to this set.
///
/// `#[schemars(inline)]` for the same reason as [`Units`]: emit the enum at every use site
/// rather than behind a shared schema reference, so reference-blind MCP clients still see the
/// allowed values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(inline)]
#[serde(rename_all = "lowercase")]
pub enum Variable {
    Temperature,
    Precipitation,
    Snowfall,
    Wind,
}

impl Variable {
    /// Sum vs mean, fixed by §1.4.
    pub fn aggregation(self) -> Aggregation {
        match self {
            Variable::Precipitation | Variable::Snowfall => Aggregation::Sum,
            Variable::Temperature | Variable::Wind => Aggregation::Mean,
        }
    }

    /// The Open-Meteo Archive `daily=` columns this variable expands to (§1.4). The first column
    /// is the one aggregated for `compare_period`; the rest ride along for `get_historical`.
    pub fn archive_columns(self) -> &'static [&'static str] {
        match self {
            Variable::Temperature => &[
                "temperature_2m_mean",
                "temperature_2m_max",
                "temperature_2m_min",
            ],
            Variable::Precipitation => &["precipitation_sum"],
            Variable::Snowfall => &["snowfall_sum"],
            Variable::Wind => &["wind_speed_10m_max"],
        }
    }

    /// The single column aggregated into the `compare_period` scalar (§1.4).
    pub fn compare_column(self) -> &'static str {
        // For temperature the compare scalar is the mean of daily *mean* temp; for the others
        // it's the variable's own daily column.
        self.archive_columns()[0]
    }

    /// The unit label for this variable under the given system (used in `comparisons[].unit`).
    pub fn unit_label(self, units: Units) -> String {
        let labels = units.labels();
        match self {
            Variable::Temperature => labels.temperature,
            Variable::Precipitation | Variable::Snowfall => labels.precipitation,
            Variable::Wind => labels.wind_speed,
        }
    }
}

/// The default `variables` selection (§1.4): temperature + precipitation.
pub fn default_variables() -> Vec<Variable> {
    vec![Variable::Temperature, Variable::Precipitation]
}

// ---------------------------------------------------------------------------------------------
// Location (§1.1 / §1.6)
// ---------------------------------------------------------------------------------------------

/// How a [`Location`] was resolved (§1.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum LocationSource {
    /// Resolved from a `location` name via the Geocoding API.
    Geocoded,
    /// `latitude`/`longitude` were supplied directly (name fields are null — no reverse geocode).
    Coordinates,
}

/// The resolved location, always echoed in the output envelope (§1.6) so a wrong "Springfield"
/// is visible and correctable. Name fields are `null` when `source == Coordinates` (§1.1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Location {
    pub name: Option<String>,
    pub admin1: Option<String>,
    pub country: Option<String>,
    pub country_code: Option<String>,
    pub latitude: f64,
    pub longitude: f64,
    pub elevation: f64,
    pub timezone: String,
    pub source: LocationSource,
}

// ---------------------------------------------------------------------------------------------
// Shared output envelope (§1.6)
// ---------------------------------------------------------------------------------------------

/// The fields every successful result carries (§1.6). Flattened into each tool's result struct
/// via `#[serde(flatten)]`, so the tool-specific payload sits alongside it.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Envelope {
    pub location: Location,
    pub units: UnitLabels,
    pub notes: Notes,
}

// ---------------------------------------------------------------------------------------------
// Error model (§1.5)
// ---------------------------------------------------------------------------------------------

/// User-actionable failure codes (§1.5). Returned inside a `CallToolResult` with
/// `is_error: true` (see [`WeatherError::to_json`]), never as a protocol-level error, so the
/// model can read and recover.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    /// Neither/both of location vs lat/lon; malformed params.
    InvalidRequest,
    /// Geocoding returned zero results.
    LocationNotFound,
    /// `start > end`; `start` before 1940-01-01; end in the future.
    InvalidDateRange,
    /// Open-Meteo HTTP 429.
    UpstreamRateLimited,
    /// Open-Meteo `{"error":true,"reason":…}` or 5xx (reason passed through in `message`).
    UpstreamError,
    /// Network failure / timeout reaching Open-Meteo.
    UpstreamUnavailable,
}

/// A structured, user-actionable error (§1.5). Carries a machine `code`, a human `message`, and
/// free-form `details`. Serializes to the `{ "error": { code, message, details } }` body via
/// [`WeatherError::to_json`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WeatherError {
    pub code: ErrorCode,
    pub message: String,
    #[serde(default)]
    pub details: serde_json::Value,
}

impl WeatherError {
    /// Construct an error with empty `details`.
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            details: serde_json::json!({}),
        }
    }

    /// Attach structured `details`.
    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = details;
        self
    }

    /// The on-the-wire body: `{ "error": { "code", "message", "details" } }` (§1.5).
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "error": {
                "code": self.code,
                "message": self.message,
                "details": self.details,
            }
        })
    }

    // -- Convenience constructors for the common codes -------------------------------------

    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::InvalidRequest, message)
    }

    pub fn location_not_found(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::LocationNotFound, message)
    }

    pub fn invalid_date_range(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::InvalidDateRange, message)
    }
}

impl std::fmt::Display for WeatherError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.code, self.message)
    }
}

impl std::error::Error for WeatherError {}
