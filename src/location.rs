//! Location resolution (tool-specs §1.1, test-plan §3.3) — pure given geocoding hits.
//!
//! Splits into a param-validation step ([`parse_location_input`], the exactly-one-of rule) and a
//! resolution step ([`resolve_geocoded`], top-by-population pick + ambiguity note). The geocoding
//! I/O itself lives behind [`crate::openmeteo::WeatherData`]; everything here is pure and
//! fixture-testable. The §3.3 tests pin the decision logic.

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
/// The §3.3 invalid_request test pins it.
pub fn parse_location_input(
    location: Option<String>,
    latitude: Option<f64>,
    longitude: Option<f64>,
) -> Result<LocationInput, WeatherError> {
    match (location, latitude, longitude) {
        // A name, with no coordinates: geocode it.
        (Some(name), None, None) => Ok(LocationInput::Name(name)),
        // Both coordinates, with no name: use them directly.
        (None, Some(latitude), Some(longitude)) => Ok(LocationInput::Coordinates {
            latitude,
            longitude,
        }),
        // Neither provided, both provided, or only one coordinate: ambiguous/underspecified.
        _ => Err(WeatherError::invalid_request(
            "provide exactly one of `location` or both `latitude` and `longitude`",
        )),
    }
}

/// Resolve geocoding hits to a single [`Location`] (`source: geocoded`) plus any `notes` (§1.1):
/// zero hits → `location_not_found`; otherwise the top result by population wins, and when more
/// than one strong match exists the alternatives are listed in `notes` (non-fatal).
///
/// The §3.3 pick/ambiguity/not-found tests pin it.
pub fn resolve_geocoded(
    hits: &[GeoHit],
    query_name: &str,
) -> Result<(Location, Notes), WeatherError> {
    if hits.is_empty() {
        return Err(WeatherError::location_not_found(format!(
            "No place matches \"{query_name}\"."
        )));
    }

    // Rank by population descending, treating a missing population as 0. `sort_by_key` is stable,
    // so equal populations keep their input order (§1.1 tie-break).
    let mut ranked: Vec<&GeoHit> = hits.iter().collect();
    ranked.sort_by_key(|h| std::cmp::Reverse(h.population.unwrap_or(0)));

    let top = ranked[0];
    let location = Location {
        name: Some(top.name.clone()),
        admin1: top.admin1.clone(),
        country: top.country.clone(),
        country_code: top.country_code.clone(),
        latitude: top.latitude,
        longitude: top.longitude,
        elevation: top.elevation,
        timezone: top.timezone.clone(),
        source: LocationSource::Geocoded,
    };

    let mut notes = Notes::new();
    if hits.len() > 1 {
        // List the top alternatives (everything but the winner), population desc. Each is
        // `"{name}, {admin1}"` so a same-named hit is distinguishable by region.
        let alternatives: Vec<String> = ranked[1..]
            .iter()
            .take(3)
            .map(|h| format!("{}, {}", h.name, h.admin1.as_deref().unwrap_or("")))
            .collect();
        notes.push(format!(
            "Multiple matches for \"{query_name}\"; using {}, {}. Alternatives: {}.",
            top.name,
            top.admin1.as_deref().unwrap_or(""),
            alternatives.join("; ")
        ));
    }

    Ok((location, notes))
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
