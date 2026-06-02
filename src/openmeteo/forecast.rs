//! Forecast API: wire types, the `get_forecast` payload (tool-specs §2), and the parse seam.
//!
//! Values arrive already in the requested units (the query sets `temperature_unit` etc.), so
//! parsing is a pure rename/reshape + WMO decode — no unit math. `parse_forecast` is stubbed in
//! Phase 2; the deserialize test (over the recorded fixture) and the `get_forecast` snapshot
//! (test-plan §3.4) pin it.

use serde::{Deserialize, Serialize};

use crate::types::WeatherError;

// ---------------------------------------------------------------------------------------------
// Domain payload (the §2 output, minus the shared envelope)
// ---------------------------------------------------------------------------------------------

/// Current conditions (§2). `weather` is the decoded WMO label for `weather_code`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Current {
    pub time: String,
    pub temperature: f64,
    pub apparent_temperature: f64,
    pub relative_humidity: i64,
    pub precipitation: f64,
    pub weather_code: u8,
    pub weather: String,
    pub wind_speed: f64,
    pub wind_direction: i64,
    pub wind_gusts: f64,
    pub cloud_cover: i64,
    pub is_day: bool,
}

/// The columnar N-day daily block (§2) — chart-friendly, all arrays length `forecast_days`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct DailyForecast {
    pub time: Vec<String>,
    pub weather_code: Vec<u8>,
    pub temperature_max: Vec<f64>,
    pub temperature_min: Vec<f64>,
    pub precipitation_sum: Vec<f64>,
    /// `null` on days Open-Meteo omits a probability.
    pub precipitation_probability_max: Vec<Option<i64>>,
    pub wind_speed_max: Vec<f64>,
}

/// The full `get_forecast` payload. `latitude`/`longitude`/`elevation`/`timezone` ride along so
/// the handler can build the envelope's `location` even when coordinates were supplied directly
/// (no geocode, §1.1).
#[derive(Debug, Clone, PartialEq)]
pub struct ForecastPayload {
    pub latitude: f64,
    pub longitude: f64,
    pub elevation: f64,
    pub timezone: String,
    pub current: Current,
    pub daily: DailyForecast,
}

// ---------------------------------------------------------------------------------------------
// Wire types (raw Open-Meteo JSON)
// ---------------------------------------------------------------------------------------------

/// Raw Forecast API response. Field names mirror the Open-Meteo wire columns requested in §2.
#[derive(Debug, Clone, Deserialize)]
pub struct RawForecast {
    pub latitude: f64,
    pub longitude: f64,
    pub elevation: f64,
    pub timezone: String,
    pub current: RawCurrent,
    pub daily: RawDaily,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawCurrent {
    pub time: String,
    pub temperature_2m: f64,
    pub apparent_temperature: f64,
    pub relative_humidity_2m: i64,
    pub precipitation: f64,
    pub weather_code: u8,
    pub wind_speed_10m: f64,
    pub wind_direction_10m: i64,
    pub wind_gusts_10m: f64,
    pub cloud_cover: i64,
    /// Open-Meteo encodes day/night as 1/0.
    pub is_day: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawDaily {
    pub time: Vec<String>,
    pub weather_code: Vec<u8>,
    pub temperature_2m_max: Vec<f64>,
    pub temperature_2m_min: Vec<f64>,
    pub precipitation_sum: Vec<f64>,
    pub precipitation_probability_max: Vec<Option<i64>>,
    pub wind_speed_10m_max: Vec<f64>,
}

/// Parse a Forecast API body into the [`ForecastPayload`]: deserialize [`RawForecast`], rename to
/// the clean §2 columns, and decode `weather_code` → `weather` via [`crate::wmo::decode`].
///
/// Phase 3 fills this in; the deserialize + snapshot tests (test-plan §3.2/§3.4) pin it.
pub fn parse_forecast(_body: &str) -> Result<ForecastPayload, WeatherError> {
    todo!("Phase 3: RawForecast -> ForecastPayload + WMO decode (test-plan §3.2/§3.4)")
}
