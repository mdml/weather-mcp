//! Parsing + error mapping (test-plan §3.2).
//!
//! Split by design: deserializing the fixtures into the **wire** structs needs no logic, so those
//! tests are green now (they prove the structs match real Open-Meteo JSON). The `parse_*` mapping
//! into domain types, the WMO decode through parsing, and the HTTP error mapping are the
//! **stubbed** logic — red until Phase 3. Unit-label/param mapping is pure data and green now.

mod common;

use common::read_fixture;
use weather_mcp::openmeteo::archive::RawArchive;
use weather_mcp::openmeteo::forecast::RawForecast;
use weather_mcp::openmeteo::{
    map_http_error, parse_archive, parse_forecast, parse_geocode, RawGeocode,
};
use weather_mcp::types::{ErrorCode, Units};

// ---- wire-struct deserialization (green: structs match the recorded JSON) ---------------------

#[test]
fn forecast_fixture_deserializes_into_wire_struct() {
    let raw: RawForecast =
        serde_json::from_str(&read_fixture("forecast_boston.json")).expect("forecast wire parse");
    assert_eq!(raw.daily.time.len(), 7);
    assert_eq!(raw.daily.temperature_2m_max.len(), 7);
    assert_eq!(raw.current.is_day, 1);
}

#[test]
fn archive_fixture_deserializes_into_wire_struct() {
    let raw: RawArchive = serde_json::from_str(&read_fixture("archive_boston_1991-2026.json"))
        .expect("archive wire parse");
    assert_eq!(raw.daily.time.len(), 12_929);
    for col in [
        "temperature_2m_max",
        "temperature_2m_min",
        "temperature_2m_mean",
        "precipitation_sum",
        "snowfall_sum",
        "wind_speed_10m_max",
    ] {
        assert!(raw.daily.columns.contains_key(col), "missing column {col}");
        assert_eq!(raw.daily.columns[col].len(), 12_929);
    }
}

#[test]
fn geocode_fixtures_deserialize() {
    let hits: RawGeocode =
        serde_json::from_str(&read_fixture("geocode_boston.json")).expect("geocode wire parse");
    assert_eq!(hits.results.as_ref().map(|r| r.len()), Some(10));

    let empty: RawGeocode = serde_json::from_str(&read_fixture("geocode_empty.json"))
        .expect("empty geocode wire parse");
    assert!(empty.results.is_none(), "no results key ⇒ None");
}

// ---- units: enum → Open-Meteo params + echoed labels (green: pure data, §1.2) -----------------

#[test]
fn units_map_to_params_and_labels() {
    let m = Units::Metric;
    assert_eq!(
        (
            m.temperature_param(),
            m.precipitation_param(),
            m.wind_speed_param()
        ),
        ("celsius", "mm", "kmh")
    );
    let ml = m.labels();
    assert_eq!(
        (
            ml.temperature.as_str(),
            ml.precipitation.as_str(),
            ml.wind_speed.as_str()
        ),
        ("°C", "mm", "km/h")
    );

    let i = Units::Imperial;
    assert_eq!(
        (
            i.temperature_param(),
            i.precipitation_param(),
            i.wind_speed_param()
        ),
        ("fahrenheit", "inch", "mph")
    );
    let il = i.labels();
    assert_eq!(
        (
            il.temperature.as_str(),
            il.precipitation.as_str(),
            il.wind_speed.as_str()
        ),
        ("°F", "inch", "mph")
    );
}

// ---- parse_* into domain types (red until Phase 3) --------------------------------------------

#[test]
fn parse_forecast_maps_current_and_daily() {
    let payload = parse_forecast(&read_fixture("forecast_boston.json")).expect("parse_forecast");
    assert_eq!(payload.current.time, "2026-06-02T09:00");
    assert_eq!(payload.current.temperature, 14.5);
    assert_eq!(payload.current.weather_code, 0);
    assert_eq!(payload.current.weather, "Clear sky"); // WMO decode through parsing
    assert!(payload.current.is_day); // 1 → true
    assert_eq!(payload.daily.time.len(), 7);
    assert_eq!(payload.daily.weather_code[0], 3);
    assert_eq!(payload.daily.temperature_max[0], 23.8);
    assert_eq!(payload.daily.temperature_min[0], 6.5);
    assert_eq!(payload.daily.precipitation_probability_max[0], Some(2));
    assert_eq!(payload.timezone, "America/New_York");
}

#[test]
fn parse_archive_maps_columns() {
    let data =
        parse_archive(&read_fixture("archive_boston_1991-2026.json")).expect("parse_archive");
    assert_eq!(data.timezone, "America/New_York");
    assert_eq!(data.daily.time.len(), 12_929);
    assert_eq!(data.daily.time[0], "1991-01-01");
    assert_eq!(data.daily.columns["temperature_2m_mean"][0], Some(-4.3));
    assert_eq!(data.daily.columns["precipitation_sum"][0], Some(0.0));
}

#[test]
fn parse_geocode_maps_hits_and_empty() {
    let hits = parse_geocode(&read_fixture("geocode_boston.json")).expect("parse_geocode");
    assert_eq!(hits.len(), 10);
    let first = &hits[0];
    assert_eq!(first.name, "Boston");
    assert_eq!(first.admin1.as_deref(), Some("Massachusetts"));
    assert_eq!(first.country_code.as_deref(), Some("US"));
    assert_eq!(first.population, Some(653_833));

    let empty = parse_geocode(&read_fixture("geocode_empty.json")).expect("parse_geocode empty");
    assert!(empty.is_empty());
}

// ---- HTTP error mapping (red until Phase 3, §1.5) ---------------------------------------------

#[test]
fn map_http_error_rate_limited() {
    assert_eq!(map_http_error(429, "").code, ErrorCode::UpstreamRateLimited);
}

#[test]
fn map_http_error_passes_reason_through() {
    let err = map_http_error(500, r#"{"error":true,"reason":"daily limit exceeded"}"#);
    assert_eq!(err.code, ErrorCode::UpstreamError);
    assert!(
        err.message.contains("daily limit exceeded"),
        "reason should pass through, got {:?}",
        err.message
    );
}

#[test]
fn map_http_error_5xx_without_body() {
    assert_eq!(map_http_error(503, "").code, ErrorCode::UpstreamError);
}
