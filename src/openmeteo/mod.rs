//! Open-Meteo client — the outermost I/O boundary (ARCHITECTURE.md).
//!
//! The [`WeatherData`] trait is the seam the rest of the crate depends on. Two impls satisfy it:
//! [`fixture::FixtureClient`] (serves recorded JSON from `tests/fixtures/`, used by the
//! deterministic tests *and* the conformance binary) and [`http::HttpClient`] (the real network
//! impl, exercised only by `just test-live`). Selecting between them at runtime is why the seam
//! is `Arc<dyn WeatherData>` (see `main.rs` / [test-plan §1](../../docs/design/test-plan.md)).
//!
//! Layering ([test-plan §1](../../docs/design/test-plan.md#1-the-test-seam--why-no-http-mock-is-needed)):
//! `HTTP → parse (&str→struct) → aggregate (compare.rs) → assemble`. The parse functions here
//! and the error mapping are the **pure** seam between raw bytes and typed data — fixture-tested,
//! no network. The §3.2 parsing tests pin them.

pub mod archive;
pub mod fixture;
pub mod forecast;
pub mod http;

use serde::Deserialize;

use crate::types::{Units, WeatherError};

pub use archive::{parse_archive, slice_archive, ArchiveDaily, ArchiveData};
pub use forecast::{parse_forecast, Current, DailyForecast, ForecastPayload};

/// A geocoding hit, parsed from the Geocoding API (`/v1/search`).
#[derive(Debug, Clone, PartialEq)]
pub struct GeoHit {
    pub name: String,
    pub admin1: Option<String>,
    pub country: Option<String>,
    pub country_code: Option<String>,
    pub latitude: f64,
    pub longitude: f64,
    pub elevation: f64,
    pub timezone: String,
    /// Population — the tie-breaker; the top result by population wins (§1.1).
    pub population: Option<u64>,
    /// Open-Meteo feature code (e.g. `PPLA`), kept for future disambiguation.
    pub feature_code: Option<String>,
}

/// A `get_forecast` upstream request (resolved coordinates + knobs). Built by the handler from
/// the validated tool params; consumed by [`WeatherData::forecast`].
#[derive(Debug, Clone, PartialEq)]
pub struct ForecastQuery {
    pub latitude: f64,
    pub longitude: f64,
    pub forecast_days: u8,
    pub units: Units,
}

/// An Archive (ERA5) request for a single date window. `columns` is the union of the requested
/// variables' Open-Meteo daily columns (see [`crate::types::Variable::archive_columns`]).
#[derive(Debug, Clone, PartialEq)]
pub struct ArchiveQuery {
    pub latitude: f64,
    pub longitude: f64,
    /// Inclusive `YYYY-MM-DD`.
    pub start_date: String,
    /// Inclusive `YYYY-MM-DD` (already clamped for the ERA5 lag, §1.7, by the caller).
    pub end_date: String,
    pub columns: Vec<String>,
    pub units: Units,
}

// ---------------------------------------------------------------------------------------------
// The I/O seam
// ---------------------------------------------------------------------------------------------

/// The Open-Meteo data source. The only async/`dyn`-dispatched seam in the crate, so the fixture
/// and HTTP impls are interchangeable. Each method maps an upstream failure to a structured
/// [`WeatherError`] (§1.5) rather than leaking transport errors.
#[async_trait::async_trait]
pub trait WeatherData: Send + Sync {
    /// Resolve a place name to candidate hits via the Geocoding API (top-by-population wins,
    /// §1.1). An empty vec means "no match" — the caller turns that into `location_not_found`.
    async fn geocode(&self, name: &str, count: u32) -> Result<Vec<GeoHit>, WeatherError>;

    /// Current conditions + an N-day daily forecast (§2).
    async fn forecast(&self, query: &ForecastQuery) -> Result<ForecastPayload, WeatherError>;

    /// Daily ERA5 archive columns for a window (§3 / the `compare_period` baseline + period).
    async fn archive(&self, query: &ArchiveQuery) -> Result<ArchiveData, WeatherError>;
}

// ---------------------------------------------------------------------------------------------
// Geocoding wire types + parse (§1.1, §3.3)
// ---------------------------------------------------------------------------------------------

/// Raw Geocoding API envelope. `results` is absent (not `[]`) when there are zero matches.
#[derive(Debug, Clone, Deserialize)]
pub struct RawGeocode {
    #[serde(default)]
    pub results: Option<Vec<RawGeoHit>>,
}

/// A raw geocoding hit as Open-Meteo returns it.
#[derive(Debug, Clone, Deserialize)]
pub struct RawGeoHit {
    pub name: String,
    pub latitude: f64,
    pub longitude: f64,
    #[serde(default)]
    pub elevation: f64,
    #[serde(default)]
    pub timezone: String,
    #[serde(default)]
    pub admin1: Option<String>,
    #[serde(default)]
    pub country: Option<String>,
    #[serde(default)]
    pub country_code: Option<String>,
    #[serde(default)]
    pub population: Option<u64>,
    #[serde(default)]
    pub feature_code: Option<String>,
}

/// Parse a Geocoding API response body into [`GeoHit`]s. A missing/empty `results` yields an
/// empty vec (the caller maps that to `location_not_found`). Malformed JSON is an
/// `upstream_error`.
///
/// The §3.2/§3.3 tests pin it.
pub fn parse_geocode(body: &str) -> Result<Vec<GeoHit>, WeatherError> {
    let raw: RawGeocode = serde_json::from_str(body).map_err(|e| {
        WeatherError::new(
            crate::types::ErrorCode::UpstreamError,
            format!("failed to parse geocode response: {e}"),
        )
    })?;

    Ok(raw
        .results
        .unwrap_or_default()
        .into_iter()
        .map(|h| GeoHit {
            name: h.name,
            admin1: h.admin1,
            country: h.country,
            country_code: h.country_code,
            latitude: h.latitude,
            longitude: h.longitude,
            elevation: h.elevation,
            timezone: h.timezone,
            population: h.population,
            feature_code: h.feature_code,
        })
        .collect())
}

// ---------------------------------------------------------------------------------------------
// Error mapping (§1.5, §3.2)
// ---------------------------------------------------------------------------------------------

/// Map an upstream HTTP response (status + body) to a structured [`WeatherError`] (§1.5):
/// 429 → `upstream_rate_limited`; an `{"error":true,"reason":…}` body or any 5xx →
/// `upstream_error` (reason passed through in `message`). A 2xx never reaches here.
///
/// Network failures / timeouts map to `upstream_unavailable` at the call site (see
/// [`http::HttpClient`]), not here. The §3.2 tests pin it.
pub fn map_http_error(status: u16, body: &str) -> WeatherError {
    use crate::types::ErrorCode;

    if status == 429 {
        return WeatherError::new(
            ErrorCode::UpstreamRateLimited,
            "Open-Meteo rate limit exceeded (HTTP 429)",
        );
    }

    if let Ok(value) = serde_json::from_str::<serde_json::Value>(body) {
        if value.get("error").and_then(serde_json::Value::as_bool) == Some(true) {
            if let Some(reason) = value.get("reason").and_then(serde_json::Value::as_str) {
                return WeatherError::new(ErrorCode::UpstreamError, reason);
            }
        }
    }

    WeatherError::new(
        ErrorCode::UpstreamError,
        format!("Open-Meteo returned HTTP {status}"),
    )
}
