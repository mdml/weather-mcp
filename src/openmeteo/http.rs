//! Real HTTP [`WeatherData`] against the live Open-Meteo endpoints (Appendix A).
//!
//! This is the **only** network-touching code and the **only** thing `just test-live` exercises
//! (test-plan §3.6). It is deliberately the *last* thing built: Phase 3 fills it in behind the
//! now-proven seam, adding the HTTP client dependency, building the request URLs from the queries,
//! mapping transport failures to `upstream_unavailable` and non-2xx responses via
//! [`crate::openmeteo::map_http_error`], then delegating to the same `parse_*` functions the
//! fixture client uses. Stubbed in Phase 2 so the seam compiles and the binary links.

use crate::openmeteo::{
    archive::ArchiveData, forecast::ForecastPayload, ArchiveQuery, ForecastQuery, GeoHit,
    WeatherData,
};
use crate::types::WeatherError;

/// The live Open-Meteo client. Phase 3 gives it an HTTP client + the endpoint base URLs.
#[derive(Debug, Clone, Default)]
pub struct HttpClient {
    // Phase 3: reqwest client + base URLs (forecast / archive / geocoding, Appendix A).
}

impl HttpClient {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait::async_trait]
impl WeatherData for HttpClient {
    async fn geocode(&self, _name: &str, _count: u32) -> Result<Vec<GeoHit>, WeatherError> {
        todo!("Phase 3: GET geocoding-api, map errors, parse_geocode (test-plan §3.6)")
    }

    async fn forecast(&self, _query: &ForecastQuery) -> Result<ForecastPayload, WeatherError> {
        todo!("Phase 3: GET api.open-meteo.com/v1/forecast, parse_forecast (test-plan §3.6)")
    }

    async fn archive(&self, _query: &ArchiveQuery) -> Result<ArchiveData, WeatherError> {
        todo!("Phase 3: GET archive-api.open-meteo.com/v1/archive, parse_archive (test-plan §3.6)")
    }
}
