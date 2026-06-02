//! Real HTTP [`WeatherData`] against the live Open-Meteo endpoints (Appendix A).
//!
//! This is the **only** network-touching code and the **only** thing `just test-live` exercises
//! (test-plan §3.6). It is deliberately the *last* thing built: Phase 3 fills it in behind the
//! now-proven seam, adding the HTTP client dependency, building the request URLs from the queries,
//! mapping transport failures to `upstream_unavailable` and non-2xx responses via
//! [`crate::openmeteo::map_http_error`], then delegating to the same `parse_*` functions the
//! fixture client uses.

use crate::openmeteo::{
    archive::ArchiveData, forecast::ForecastPayload, map_http_error, parse_archive, parse_forecast,
    parse_geocode, ArchiveQuery, ForecastQuery, GeoHit, WeatherData,
};
use crate::types::{ErrorCode, Units, WeatherError};

/// Forecast endpoint (Appendix A).
const FORECAST_URL: &str = "https://api.open-meteo.com/v1/forecast";
/// Archive (ERA5) endpoint (Appendix A).
const ARCHIVE_URL: &str = "https://archive-api.open-meteo.com/v1/archive";
/// Geocoding endpoint (Appendix A).
const GEOCODE_URL: &str = "https://geocoding-api.open-meteo.com/v1/search";

/// The exact `current=` wire columns requested for `get_forecast` (tool-specs §2). Every name
/// here has a matching field in [`crate::openmeteo::forecast::RawCurrent`].
const FORECAST_CURRENT: &str = "temperature_2m,relative_humidity_2m,apparent_temperature,\
precipitation,weather_code,wind_speed_10m,wind_direction_10m,wind_gusts_10m,cloud_cover,is_day";

/// The exact `daily=` wire columns requested for `get_forecast` (tool-specs §2). Every name here
/// has a matching field in [`crate::openmeteo::forecast::RawDaily`].
const FORECAST_DAILY: &str = "weather_code,temperature_2m_max,temperature_2m_min,\
precipitation_sum,precipitation_probability_max,wind_speed_10m_max";

/// The live Open-Meteo client: one shared [`reqwest::Client`] over the three Appendix-A endpoints.
#[derive(Debug, Clone)]
pub struct HttpClient {
    client: reqwest::Client,
}

impl HttpClient {
    /// Build the client once (the `reqwest::Client` holds a connection pool; reuse it).
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    /// GET `url` with `params`, mapping transport failures to `upstream_unavailable` and any
    /// non-2xx status (with its body) through [`map_http_error`]. Returns the 2xx body text for a
    /// `parse_*` to consume — no parsing happens here.
    async fn get_body(&self, url: &str, params: &[(&str, String)]) -> Result<String, WeatherError> {
        let response = self
            .client
            .get(url)
            .query(params)
            .send()
            .await
            .map_err(|e| {
                WeatherError::new(
                    ErrorCode::UpstreamUnavailable,
                    format!("failed to reach Open-Meteo: {e}"),
                )
            })?;

        let status = response.status();
        if !status.is_success() {
            // Best-effort body read so `{"error":true,"reason":…}` reasons pass through (§1.5).
            let body = response.text().await.unwrap_or_default();
            return Err(map_http_error(status.as_u16(), &body));
        }

        response.text().await.map_err(|e| {
            WeatherError::new(
                ErrorCode::UpstreamUnavailable,
                format!("failed to read Open-Meteo response body: {e}"),
            )
        })
    }
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new()
    }
}

/// The three Open-Meteo unit params for a [`Units`] system (§1.2). Shared by forecast + archive.
fn unit_params(units: Units) -> [(&'static str, String); 3] {
    [
        ("temperature_unit", units.temperature_param().to_string()),
        (
            "precipitation_unit",
            units.precipitation_param().to_string(),
        ),
        ("wind_speed_unit", units.wind_speed_param().to_string()),
    ]
}

#[async_trait::async_trait]
impl WeatherData for HttpClient {
    async fn geocode(&self, name: &str, count: u32) -> Result<Vec<GeoHit>, WeatherError> {
        let params = [
            ("name", name.to_string()),
            ("count", count.to_string()),
            ("format", "json".to_string()),
        ];
        let body = self.get_body(GEOCODE_URL, &params).await?;
        parse_geocode(&body)
    }

    async fn forecast(&self, query: &ForecastQuery) -> Result<ForecastPayload, WeatherError> {
        let mut params = vec![
            ("latitude", query.latitude.to_string()),
            ("longitude", query.longitude.to_string()),
            ("timezone", "auto".to_string()),
            ("forecast_days", query.forecast_days.to_string()),
            ("current", FORECAST_CURRENT.to_string()),
            ("daily", FORECAST_DAILY.to_string()),
        ];
        params.extend(unit_params(query.units));
        let body = self.get_body(FORECAST_URL, &params).await?;
        parse_forecast(&body)
    }

    async fn archive(&self, query: &ArchiveQuery) -> Result<ArchiveData, WeatherError> {
        let mut params = vec![
            ("latitude", query.latitude.to_string()),
            ("longitude", query.longitude.to_string()),
            ("start_date", query.start_date.clone()),
            ("end_date", query.end_date.clone()),
            ("timezone", "auto".to_string()),
            ("daily", query.columns.join(",")),
        ];
        params.extend(unit_params(query.units));
        let body = self.get_body(ARCHIVE_URL, &params).await?;
        parse_archive(&body)
    }
}
