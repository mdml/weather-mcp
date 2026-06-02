//! Fixture-backed [`WeatherData`] — serves recorded JSON from a directory (`tests/fixtures/`).
//!
//! This is the impl the deterministic suite and the conformance binary run against, so the whole
//! tool path is offline and reproducible ([test-plan §1](../../docs/design/test-plan.md)). It is
//! **real glue** (file routing + windowed archive slicing); the parsing it delegates to is the
//! part stubbed in Phase 2. Routing is convention-based against the Boston fixture set:
//!
//! - `geocode(name)` → `geocode_<slug>.json`, falling back to `geocode_empty.json` for any name
//!   without a fixture (so an arbitrary nonsense name exercises the `location_not_found` path).
//! - `forecast(..)` → [`FORECAST_FIXTURE`].
//! - `archive(query)` → [`ARCHIVE_FIXTURE`], parsed once and sliced to the requested window.

use std::path::{Path, PathBuf};

use crate::openmeteo::{
    archive::{parse_archive, slice_archive, ArchiveData},
    forecast::{parse_forecast, ForecastPayload},
    ArchiveQuery, ForecastQuery, GeoHit, WeatherData,
};
use crate::types::{ErrorCode, WeatherError};

/// The single forecast fixture (Boston, 7-day) all forecast queries resolve to.
pub const FORECAST_FIXTURE: &str = "forecast_boston.json";
/// The wide archive fixture (Boston, 1991→2026) every archive window is sliced from.
pub const ARCHIVE_FIXTURE: &str = "archive_boston_1991-2026.json";
/// Fallback for a geocode name with no dedicated fixture (zero results).
pub const GEOCODE_EMPTY_FIXTURE: &str = "geocode_empty.json";

/// A [`WeatherData`] that reads recorded responses from a fixtures directory.
#[derive(Debug, Clone)]
pub struct FixtureClient {
    dir: PathBuf,
}

impl FixtureClient {
    /// Build a client rooted at `dir` (e.g. `tests/fixtures`).
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    async fn read(&self, file: &str) -> Result<String, WeatherError> {
        let path = self.dir.join(file);
        tokio::fs::read_to_string(&path).await.map_err(|e| {
            WeatherError::new(
                ErrorCode::UpstreamUnavailable,
                format!("fixture {} unreadable: {e}", path.display()),
            )
        })
    }

    /// Read a geocode fixture for `name`, falling back to the empty fixture.
    async fn read_geocode(&self, name: &str) -> Result<String, WeatherError> {
        let file = format!("geocode_{}.json", slugify(name));
        if self.dir.join(&file).exists() {
            self.read(&file).await
        } else {
            self.read(GEOCODE_EMPTY_FIXTURE).await
        }
    }
}

#[async_trait::async_trait]
impl WeatherData for FixtureClient {
    async fn geocode(&self, name: &str, _count: u32) -> Result<Vec<GeoHit>, WeatherError> {
        let body = self.read_geocode(name).await?;
        crate::openmeteo::parse_geocode(&body)
    }

    async fn forecast(&self, _query: &ForecastQuery) -> Result<ForecastPayload, WeatherError> {
        let body = self.read(FORECAST_FIXTURE).await?;
        parse_forecast(&body)
    }

    async fn archive(&self, query: &ArchiveQuery) -> Result<ArchiveData, WeatherError> {
        let body = self.read(ARCHIVE_FIXTURE).await?;
        let full = parse_archive(&body)?;
        Ok(slice_archive(&full, &query.start_date, &query.end_date))
    }
}

/// Lowercase + collapse non-alphanumerics to `_` (so `"Boston, MA"` → `boston_ma`).
fn slugify(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut prev_us = false;
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            prev_us = false;
        } else if !prev_us {
            out.push('_');
            prev_us = true;
        }
    }
    out.trim_matches('_').to_string()
}

/// The default fixtures directory relative to the crate root, for the binary's fixture mode.
pub fn default_dir() -> &'static Path {
    Path::new("tests/fixtures")
}

#[cfg(test)]
mod tests {
    use super::slugify;

    #[test]
    fn slugify_handles_names_and_punctuation() {
        assert_eq!(slugify("Boston"), "boston");
        assert_eq!(slugify("Boston, MA"), "boston_ma");
        assert_eq!(slugify("Reykjavík"), "reykjav_k"); // non-ascii collapses
        assert_eq!(slugify("  Nowhere  "), "nowhere");
    }
}
