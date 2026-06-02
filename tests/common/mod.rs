//! Shared test helpers. Included via `mod common;` in each integration test.
//!
//! Crucially, the archive loader/slicer here are **independent of the code under test**: they
//! deserialize the fixture and reshape/slice it directly, so the `compare` oracle tests don't
//! depend on the (stubbed) `parse_archive`/`slice_archive`. That independence is the whole point
//! of the oracle (test-plan §3.1) — the expected numbers and the inputs are computed without the
//! implementation.

#![allow(dead_code)] // not every test file uses every helper

use std::path::{Path, PathBuf};

use weather_mcp::openmeteo::archive::{ArchiveDaily, ArchiveData, RawArchive};
use weather_mcp::openmeteo::{GeoHit, RawGeocode};

pub fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

pub fn read_fixture(name: &str) -> String {
    let path = fixtures_dir().join(name);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read fixture {}: {e}", path.display()))
}

/// The wide Boston archive fixture as [`ArchiveData`], loaded without `parse_archive`.
pub fn archive_fixture() -> ArchiveData {
    let raw: RawArchive =
        serde_json::from_str(&read_fixture("archive_boston_1991-2026.json")).expect("archive json");
    ArchiveData {
        latitude: raw.latitude,
        longitude: raw.longitude,
        elevation: raw.elevation,
        timezone: raw.timezone,
        daily: ArchiveDaily {
            time: raw.daily.time,
            columns: raw.daily.columns,
        },
    }
}

/// Inclusive `YYYY-MM-DD` window slice, preserving index alignment (test-local twin of the
/// stubbed `slice_archive`).
pub fn slice(data: &ArchiveData, start: &str, end: &str) -> ArchiveData {
    let keep: Vec<usize> = data
        .daily
        .time
        .iter()
        .enumerate()
        .filter(|(_, d)| d.as_str() >= start && d.as_str() <= end)
        .map(|(i, _)| i)
        .collect();
    let time = keep.iter().map(|&i| data.daily.time[i].clone()).collect();
    let columns = data
        .daily
        .columns
        .iter()
        .map(|(k, v)| (k.clone(), keep.iter().map(|&i| v[i]).collect()))
        .collect();
    ArchiveData {
        daily: ArchiveDaily { time, columns },
        ..data.clone()
    }
}

/// Load geocode hits from a fixture into [`GeoHit`]s, independent of the stubbed `parse_geocode`.
pub fn geocode_hits(fixture: &str) -> Vec<GeoHit> {
    let raw: RawGeocode = serde_json::from_str(&read_fixture(fixture)).expect("geocode json");
    raw.results
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
        .collect()
}

/// Build a small crafted [`ArchiveData`] from `(date, value)` pairs for one column — used for the
/// cross-year and Feb-29 edge tests where a precise, hand-checkable dataset beats the fixture.
pub fn crafted(column: &str, days: &[(&str, f64)]) -> ArchiveData {
    let mut columns = std::collections::BTreeMap::new();
    columns.insert(
        column.to_string(),
        days.iter().map(|(_, v)| Some(*v)).collect(),
    );
    ArchiveData {
        latitude: 0.0,
        longitude: 0.0,
        elevation: 0.0,
        timezone: "UTC".to_string(),
        daily: ArchiveDaily {
            time: days.iter().map(|(d, _)| d.to_string()).collect(),
            columns,
        },
    }
}

#[track_caller]
pub fn approx(actual: f64, expected: f64, eps: f64) {
    assert!(
        (actual - expected).abs() <= eps,
        "expected ~{expected}, got {actual} (eps {eps})"
    );
}
