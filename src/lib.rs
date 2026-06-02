//! `weather-mcp` library — the testable surface of the server.
//!
//! A single small crate (ARCHITECTURE.md): the binary (`main.rs`) is a thin shell that selects a
//! transport + data source and serves; everything worth testing lives here and is reached by the
//! `tests/` integration tests and the in-module unit tests.
//!
//! Module map:
//! - [`types`] — shared contract vocabulary (units, variables, location, envelope, error model).
//! - [`wmo`] — the WMO weather-code decode table.
//! - [`openmeteo`] — the I/O boundary: the [`openmeteo::WeatherData`] seam + fixture/HTTP impls +
//!   the pure parse functions.
//! - [`location`] — location resolution (validate params, pick a geocode hit).
//! - [`compare`] — the `compare_period` aggregation (pure, fixture-tested).
//! - [`server`] — the rmcp `ServerHandler` and the three tools.

pub mod compare;
pub mod dates;
pub mod location;
pub mod openmeteo;
pub mod server;
pub mod types;
pub mod wmo;

use std::sync::Arc;

use crate::openmeteo::{fixture::FixtureClient, http::HttpClient, WeatherData};

/// The env var that switches the binary onto the fixture-backed client (offline, deterministic).
/// Set to a fixtures directory; the conformance test and `just test` use it. When unset, the
/// binary uses the real HTTP client (Phase 3).
pub const FIXTURES_ENV: &str = "WEATHER_MCP_FIXTURES";

/// Build the data source the binary serves over, from the environment ([`FIXTURES_ENV`]).
pub fn client_from_env() -> Arc<dyn WeatherData> {
    match std::env::var(FIXTURES_ENV) {
        Ok(dir) if !dir.is_empty() => Arc::new(FixtureClient::new(dir)),
        _ => Arc::new(HttpClient::new()),
    }
}
