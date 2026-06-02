//! Location resolution (tool-specs §1.1, test-plan §3.3) — pure given geocoding hits.
//!
//! Splits into a param-validation step ([`parse_location_input`], the exactly-one-of rule) and a
//! resolution step ([`resolve_geocoded`], top-by-population pick + ambiguity note). The geocoding
//! I/O itself lives behind [`crate::openmeteo::WeatherData`]; everything here is pure and
//! fixture-testable. The decision logic is stubbed in Phase 2; the §3.3 tests are the red bar.

use crate::openmeteo::GeoHit;
use crate::types::{Location, LocationSource, Notes, WeatherError};

/// Where a request's location came from, after validating the exactly-one-of rule (§1.1).
#[derive(Debug, Clone, PartialEq)]
pub enum LocationInput {
    /// A place name to geocode.
    Name(String),
    /// Coordinates supplied directly (the unambiguous escape hatch).
    Coordinates { latitude: f64, longitude: f64 },
}

/// Validate the location params: **exactly one** of `location` vs `latitude`+`longitude` (§1.1).
/// Supplying neither, both, or only one coordinate is `invalid_request`.
///
/// Phase 3 fills this in; the §3.3 invalid_request test pins it.
pub fn parse_location_input(
    _location: Option<String>,
    _latitude: Option<f64>,
    _longitude: Option<f64>,
) -> Result<LocationInput, WeatherError> {
    todo!("Phase 3: enforce exactly-one-of location vs lat/lon (test-plan §3.3)")
}

/// Resolve geocoding hits to a single [`Location`] (`source: geocoded`) plus any `notes` (§1.1):
/// zero hits → `location_not_found`; otherwise the top result by population wins, and when more
/// than one strong match exists the alternatives are listed in `notes` (non-fatal).
///
/// Phase 3 fills this in; the §3.3 pick/ambiguity/not-found tests pin it.
pub fn resolve_geocoded(
    _hits: &[GeoHit],
    _query_name: &str,
) -> Result<(Location, Notes), WeatherError> {
    todo!("Phase 3: top-by-population pick + alternatives note + not-found (test-plan §3.3)")
}

/// Build a [`Location`] for directly-supplied coordinates (§1.1): `source: coordinates`, name
/// fields null (no reverse geocoding in v1), timezone + elevation taken from the weather response.
/// A pure constructor — no resolution decisions, so it's implemented now.
pub fn location_from_coordinates(
    latitude: f64,
    longitude: f64,
    elevation: f64,
    timezone: impl Into<String>,
) -> Location {
    Location {
        name: None,
        admin1: None,
        country: None,
        country_code: None,
        latitude,
        longitude,
        elevation,
        timezone: timezone.into(),
        source: LocationSource::Coordinates,
    }
}
