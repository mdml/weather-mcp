//! Location resolution (test-plan §3.3).
//!
//! The geocode hits are loaded independently of the stubbed `parse_geocode`. `resolve_geocoded`
//! and `parse_location_input` are the stubbed decision logic (red until Phase 3);
//! `location_from_coordinates` is a pure constructor (green now).

mod common;

use common::geocode_hits;
use weather_mcp::location::{
    location_from_coordinates, parse_location_input, resolve_geocoded, LocationInput,
};
use weather_mcp::types::{ErrorCode, LocationSource};

// ---- name → coords, top-by-population pick, place echoed (red) ---------------------------------

#[test]
fn resolves_top_by_population_and_echoes_place() {
    let hits = geocode_hits("geocode_boston.json");
    let (loc, _notes) = resolve_geocoded(&hits, "Boston").expect("resolves");
    assert_eq!(loc.source, LocationSource::Geocoded);
    assert_eq!(loc.name.as_deref(), Some("Boston"));
    assert_eq!(loc.admin1.as_deref(), Some("Massachusetts")); // pop 653,833 wins
    assert_eq!(loc.country_code.as_deref(), Some("US"));
    assert!((loc.latitude - 42.358_43).abs() < 1e-4);
}

#[test]
fn multiple_strong_matches_are_noted() {
    let hits = geocode_hits("geocode_boston.json");
    let (_loc, notes) = resolve_geocoded(&hits, "Boston").expect("resolves");
    assert!(!notes.is_empty(), "expected an ambiguity note");
    assert!(
        notes.iter().any(|n| n.contains("New York")),
        "alternatives should mention the runner-up Boston, got {notes:?}"
    );
}

#[test]
fn zero_matches_is_location_not_found() {
    let err = resolve_geocoded(&[], "Atlantis").expect_err("no hits ⇒ error");
    assert_eq!(err.code, ErrorCode::LocationNotFound);
}

// ---- param validation: exactly-one-of (red) ---------------------------------------------------

#[test]
fn coordinates_parse_to_coordinate_input() {
    let input = parse_location_input(None, Some(42.0), Some(-71.0)).expect("valid coords");
    assert_eq!(
        input,
        LocationInput::Coordinates {
            latitude: 42.0,
            longitude: -71.0
        }
    );
}

#[test]
fn name_parses_to_name_input() {
    let input = parse_location_input(Some("Boston".to_string()), None, None).expect("valid name");
    assert_eq!(input, LocationInput::Name("Boston".to_string()));
}

#[test]
fn neither_location_nor_coords_is_invalid_request() {
    let err = parse_location_input(None, None, None).expect_err("neither ⇒ error");
    assert_eq!(err.code, ErrorCode::InvalidRequest);
}

#[test]
fn both_location_and_coords_is_invalid_request() {
    let err = parse_location_input(Some("Boston".to_string()), Some(42.0), Some(-71.0))
        .expect_err("both ⇒ error");
    assert_eq!(err.code, ErrorCode::InvalidRequest);
}

#[test]
fn lone_latitude_is_invalid_request() {
    let err = parse_location_input(None, Some(42.0), None).expect_err("half coords ⇒ error");
    assert_eq!(err.code, ErrorCode::InvalidRequest);
}

// ---- coordinates → Location: source + null names (green constructor, §1.1) --------------------

#[test]
fn coordinates_location_has_null_names_and_coordinate_source() {
    let loc = location_from_coordinates(42.0, -71.0, 38.0, "America/New_York");
    assert_eq!(loc.source, LocationSource::Coordinates);
    assert!(loc.name.is_none());
    assert!(loc.admin1.is_none());
    assert!(loc.country.is_none());
    assert_eq!(loc.timezone, "America/New_York");
    assert_eq!(loc.elevation, 38.0);
}
