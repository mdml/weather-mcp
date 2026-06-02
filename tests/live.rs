//! Live smoke — the real Open-Meteo API (test-plan §3.6).
//!
//! These are the **only** network-touching tests: one real `forecast`, one real `archive`, one
//! real `geocode`, asserting only that the shape parses through [`HttpClient`] + the `parse_*`
//! seam. They catch upstream drift (renamed columns, changed envelopes) that the fixture suite
//! can't. They're thin and separate by design ([test-plan §3.6](docs/design/test-plan.md)).
//!
//! Every test is `#[ignore]` so `cargo nextest run` / `just check` never touch the network. Run
//! them deliberately via `just test-live` (which passes `--run-ignored all`).

use weather_mcp::openmeteo::http::HttpClient;
use weather_mcp::openmeteo::{ArchiveQuery, ForecastQuery, WeatherData};
use weather_mcp::types::{Units, Variable};

/// Boston — the same coordinates the recorded fixtures use, so the live shape lines up with what
/// the deterministic suite already pins.
const BOSTON_LAT: f64 = 42.36;
const BOSTON_LON: f64 = -71.06;

#[tokio::test]
#[ignore = "network: run via `just test-live`"]
async fn live_forecast_parses() {
    let client = HttpClient::new();
    let payload = client
        .forecast(&ForecastQuery {
            latitude: BOSTON_LAT,
            longitude: BOSTON_LON,
            forecast_days: 7,
            units: Units::Metric,
        })
        .await
        .expect("live forecast should succeed");

    // `current` populated (a real `time` came back) + the daily block is exactly 7 days.
    assert!(
        !payload.current.time.is_empty(),
        "current.time should be populated"
    );
    assert_eq!(
        payload.daily.time.len(),
        7,
        "forecast_days=7 ⇒ 7 daily rows, got {}",
        payload.daily.time.len()
    );
    // Columns are index-aligned with `time`.
    assert_eq!(payload.daily.temperature_max.len(), 7);
    assert_eq!(payload.daily.temperature_min.len(), 7);
}

#[tokio::test]
#[ignore = "network: run via `just test-live`"]
async fn live_archive_parses_and_has_temperature_mean() {
    let client = HttpClient::new();
    // A short, recent-but-ERA5-lagged past window. Temperature columns, so we can assert the
    // verify-at-build flag (§1.4): `temperature_2m_mean` is actually served by the Archive API.
    let columns: Vec<String> = Variable::Temperature
        .archive_columns()
        .iter()
        .map(|s| s.to_string())
        .collect();
    let data = client
        .archive(&ArchiveQuery {
            latitude: BOSTON_LAT,
            longitude: BOSTON_LON,
            start_date: "2024-01-01".to_string(),
            end_date: "2024-01-07".to_string(),
            columns,
            units: Units::Metric,
        })
        .await
        .expect("live archive should succeed");

    assert_eq!(
        data.daily.time.len(),
        7,
        "2024-01-01..2024-01-07 inclusive ⇒ 7 days, got {}",
        data.daily.time.len()
    );

    // §1.4 verify-at-build flag: `temperature_2m_mean` must be present and non-empty (else the
    // compare path would have to fall back to (max+min)/2).
    let mean = data
        .daily
        .columns
        .get("temperature_2m_mean")
        .expect("Archive API should serve temperature_2m_mean (§1.4 verify-at-build flag)");
    assert_eq!(mean.len(), 7, "temperature_2m_mean should be index-aligned");
    assert!(
        mean.iter().any(Option::is_some),
        "temperature_2m_mean should have at least one real value"
    );
}

#[tokio::test]
#[ignore = "network: run via `just test-live`"]
async fn live_geocode_finds_boston() {
    let client = HttpClient::new();
    let hits = client
        .geocode("Boston", 10)
        .await
        .expect("live geocode should succeed");

    assert!(!hits.is_empty(), "geocode(\"Boston\") should return hits");
    // Top result should be a plausible Boston (name match; the US one is the population winner).
    let top = &hits[0];
    assert_eq!(top.name, "Boston");
    assert!(
        (top.latitude - BOSTON_LAT).abs() < 1.0 && (top.longitude - BOSTON_LON).abs() < 1.0,
        "top Boston should be near {BOSTON_LAT},{BOSTON_LON}, got {},{}",
        top.latitude,
        top.longitude
    );
}
