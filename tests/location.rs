//! Location resolution (test-plan §3.3).
//!
//! The geocode hits are loaded independently of `parse_geocode`. `resolve_geocoded`
//! and `parse_location_input` are the decision logic under test;
//! `location_from_coordinates` is a pure constructor.

mod common;

use common::geocode_hits;
use weather_mcp::location::{
    location_from_coordinates, parse_location_input, parse_place_query, resolve_geocoded,
    LocationInput,
};
use weather_mcp::openmeteo::GeoHit;
use weather_mcp::types::{ErrorCode, LocationSource};

/// A crafted geocoding hit for the qualifier-resolution tests. Only the fields the resolution
/// logic reads (`name`, `admin1`, `country`, `country_code`, `population`) carry meaningful values;
/// the rest are filler. `GeoHit`'s fields are public, so this builds them inline (no fixtures).
fn hit(name: &str, admin1: &str, country: &str, country_code: &str, population: u64) -> GeoHit {
    GeoHit {
        name: name.to_string(),
        admin1: Some(admin1.to_string()),
        country: Some(country.to_string()),
        country_code: Some(country_code.to_string()),
        latitude: 0.0,
        longitude: 0.0,
        elevation: 0.0,
        timezone: "UTC".to_string(),
        population: Some(population),
        feature_code: Some("PPL".to_string()),
    }
}

// ---- name → coords, top-by-population pick, place echoed (red) ---------------------------------

#[test]
fn resolves_top_by_population_and_echoes_place() {
    let hits = geocode_hits("geocode_boston.json");
    let (loc, _notes) = resolve_geocoded(&hits, "Boston", None).expect("resolves");
    assert_eq!(loc.source, LocationSource::Geocoded);
    assert_eq!(loc.name.as_deref(), Some("Boston"));
    assert_eq!(loc.admin1.as_deref(), Some("Massachusetts")); // pop 653,833 wins
    assert_eq!(loc.country_code.as_deref(), Some("US"));
    assert!((loc.latitude - 42.358_43).abs() < 1e-4);
}

#[test]
fn multiple_strong_matches_are_noted() {
    let hits = geocode_hits("geocode_boston.json");
    let (_loc, notes) = resolve_geocoded(&hits, "Boston", None).expect("resolves");
    assert!(!notes.is_empty(), "expected an ambiguity note");
    assert!(
        notes.iter().any(|n| n.contains("New York")),
        "alternatives should mention the runner-up Boston, got {notes:?}"
    );
}

#[test]
fn zero_matches_is_location_not_found() {
    let err = resolve_geocoded(&[], "Atlantis", None).expect_err("no hits ⇒ error");
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

// ---- "City, ST" split: bare name for the geocoder + a disambiguating qualifier (§1.1) ----------

#[test]
fn parse_place_query_splits_name_and_qualifier() {
    let pq = parse_place_query("Silver Spring, MD");
    assert_eq!(pq.name, "Silver Spring");
    assert_eq!(pq.admin.as_deref(), Some("MD"));
}

#[test]
fn parse_place_query_bare_name_has_no_qualifier() {
    let pq = parse_place_query("Boston");
    assert_eq!(pq.name, "Boston");
    assert_eq!(pq.admin, None);
}

#[test]
fn parse_place_query_trims_whitespace() {
    let pq = parse_place_query("  Silver Spring  ,   Maryland  ");
    assert_eq!(pq.name, "Silver Spring");
    assert_eq!(pq.admin.as_deref(), Some("Maryland"));

    // A trailing comma with an empty remainder is not a qualifier.
    let pq = parse_place_query("Boston,  ");
    assert_eq!(pq.name, "Boston");
    assert_eq!(pq.admin, None);
}

// ---- qualifier-aware resolution: the qualifier overrides top-by-population (the crux) ----------

#[test]
fn qualifier_overrides_population_via_abbrev() {
    // Springfield, MO (pop 169k) is more populous than Springfield, IL (pop 116k), but the
    // `IL` qualifier must win it for Illinois.
    let hits = vec![
        hit("Springfield", "Missouri", "United States", "US", 169_176),
        hit("Springfield", "Illinois", "United States", "US", 116_250),
    ];
    let (loc, _notes) = resolve_geocoded(&hits, "Springfield", Some("IL")).expect("resolves");
    assert_eq!(loc.admin1.as_deref(), Some("Illinois"));
}

#[test]
fn md_qualifier_picks_maryland_over_more_populous_pennsylvania() {
    let hits = vec![
        hit(
            "Silver Spring",
            "Pennsylvania",
            "United States",
            "US",
            25_000,
        ),
        hit("Silver Spring", "Maryland", "United States", "US", 81_816),
    ];
    // Even though Maryland happens to be more populous here, assert the qualifier path itself.
    let (loc, _notes) = resolve_geocoded(&hits, "Silver Spring", Some("MD")).expect("resolves");
    assert_eq!(loc.admin1.as_deref(), Some("Maryland"));

    // And a PA qualifier picks Pennsylvania, the *less* populous hit — proving it's the qualifier,
    // not population, driving the choice.
    let (loc, _notes) = resolve_geocoded(&hits, "Silver Spring", Some("PA")).expect("resolves");
    assert_eq!(loc.admin1.as_deref(), Some("Pennsylvania"));
}

#[test]
fn full_state_name_qualifier_matches() {
    let hits = vec![
        hit("Springfield", "Missouri", "United States", "US", 169_176),
        hit(
            "Springfield",
            "Massachusetts",
            "United States",
            "US",
            155_929,
        ),
    ];
    let (loc, _notes) =
        resolve_geocoded(&hits, "Springfield", Some("Massachusetts")).expect("resolves");
    assert_eq!(loc.admin1.as_deref(), Some("Massachusetts"));
}

#[test]
fn country_qualifier_matches() {
    // Two "Paris" hits; the France qualifier (matching `country`) picks the French one over the
    // more-populous nothing-special default.
    let hits = vec![
        hit("Paris", "Texas", "United States", "US", 25_171),
        hit("Paris", "Île-de-France", "France", "FR", 2_138_551),
    ];
    let (loc, _notes) = resolve_geocoded(&hits, "Paris", Some("France")).expect("resolves");
    assert_eq!(loc.country.as_deref(), Some("France"));

    // The two-letter country code matches too.
    let (loc, _notes) = resolve_geocoded(&hits, "Paris", Some("US")).expect("resolves");
    assert_eq!(loc.country_code.as_deref(), Some("US"));
}

#[test]
fn unmatched_qualifier_falls_back_to_top_by_population_with_note() {
    let hits = vec![
        hit("Springfield", "Missouri", "United States", "US", 169_176),
        hit("Springfield", "Illinois", "United States", "US", 116_250),
    ];
    // `ZZ` matches no admin1/country/country_code: non-fatal fallback to the global top + a note.
    let (loc, notes) = resolve_geocoded(&hits, "Springfield", Some("ZZ")).expect("resolves");
    assert_eq!(loc.admin1.as_deref(), Some("Missouri")); // top by population
    assert!(
        notes
            .iter()
            .any(|n| n.contains("ZZ") && n.contains("didn't match")),
        "expected a non-fatal fallback note, got {notes:?}"
    );
}

#[test]
fn no_qualifier_path_unchanged_for_bare_boston() {
    // The original no-qualifier behavior must be byte-identical: bare "Boston" still resolves to
    // Massachusetts (the top-by-population hit) and still emits the ambiguity note.
    let hits = geocode_hits("geocode_boston.json");
    let (loc, notes) = resolve_geocoded(&hits, "Boston", None).expect("resolves");
    assert_eq!(loc.admin1.as_deref(), Some("Massachusetts"));
    assert!(!notes.is_empty(), "expected the unchanged ambiguity note");
}
