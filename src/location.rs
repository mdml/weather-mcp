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

/// A `location` string split into the part Open-Meteo's geocoder understands and the qualifier we
/// disambiguate with client-side (§1.1). Open-Meteo's `/v1/search` matches only the bare place
/// name and returns zero results for a `", ST"`/`", State"`/`", Country"` suffix, so we query with
/// [`name`](Self::name) and rank the hits with [`admin`](Self::admin) ourselves.
#[derive(Debug, Clone, PartialEq)]
pub struct PlaceQuery {
    /// The bare place name to send to the geocoder (text before the first comma, trimmed).
    pub name: String,
    /// The qualifier (text after the first comma, trimmed), or `None` when there was no comma /
    /// the remainder was empty. Used to prefer a hit by `admin1`/`country`/`country_code`.
    pub admin: Option<String>,
}

/// Split a `location` string into a bare geocoder name + an optional disambiguating qualifier
/// (§1.1). Splits on the **first** comma: the part before it is the `name`; the remainder, if
/// non-empty after trimming, is the `admin` qualifier. With no comma the whole (trimmed) string is
/// the `name` and `admin` is `None`. This is what makes `"Boston, MA"` (a §1.1 valid input)
/// actually geocode: we send `"Boston"` upstream and match `"MA"` against the returned hits.
pub fn parse_place_query(input: &str) -> PlaceQuery {
    match input.split_once(',') {
        Some((name, rest)) => {
            let admin = rest.trim();
            PlaceQuery {
                name: name.trim().to_string(),
                admin: (!admin.is_empty()).then(|| admin.to_string()),
            }
        }
        None => PlaceQuery {
            name: input.trim().to_string(),
            admin: None,
        },
    }
}

/// US state/territory abbreviation → full name (50 states + DC), used to expand a qualifier like
/// `MD` → `Maryland` so it matches Open-Meteo's `admin1` (which carries the full state name).
/// Kept as static data; the lookup is case-insensitive (callers upper-case the key).
const US_STATE_ABBREVS: &[(&str, &str)] = &[
    ("AL", "Alabama"),
    ("AK", "Alaska"),
    ("AZ", "Arizona"),
    ("AR", "Arkansas"),
    ("CA", "California"),
    ("CO", "Colorado"),
    ("CT", "Connecticut"),
    ("DE", "Delaware"),
    ("DC", "District of Columbia"),
    ("FL", "Florida"),
    ("GA", "Georgia"),
    ("HI", "Hawaii"),
    ("ID", "Idaho"),
    ("IL", "Illinois"),
    ("IN", "Indiana"),
    ("IA", "Iowa"),
    ("KS", "Kansas"),
    ("KY", "Kentucky"),
    ("LA", "Louisiana"),
    ("ME", "Maine"),
    ("MD", "Maryland"),
    ("MA", "Massachusetts"),
    ("MI", "Michigan"),
    ("MN", "Minnesota"),
    ("MS", "Mississippi"),
    ("MO", "Missouri"),
    ("MT", "Montana"),
    ("NE", "Nebraska"),
    ("NV", "Nevada"),
    ("NH", "New Hampshire"),
    ("NJ", "New Jersey"),
    ("NM", "New Mexico"),
    ("NY", "New York"),
    ("NC", "North Carolina"),
    ("ND", "North Dakota"),
    ("OH", "Ohio"),
    ("OK", "Oklahoma"),
    ("OR", "Oregon"),
    ("PA", "Pennsylvania"),
    ("RI", "Rhode Island"),
    ("SC", "South Carolina"),
    ("SD", "South Dakota"),
    ("TN", "Tennessee"),
    ("TX", "Texas"),
    ("UT", "Utah"),
    ("VT", "Vermont"),
    ("VA", "Virginia"),
    ("WA", "Washington"),
    ("WV", "West Virginia"),
    ("WI", "Wisconsin"),
    ("WY", "Wyoming"),
];

/// Expand a US state abbreviation (e.g. `"md"`) to its full name (`"Maryland"`), case-insensitively.
/// Returns `None` for anything that isn't a known abbreviation.
fn expand_us_state(token: &str) -> Option<&'static str> {
    let key = token.trim().to_ascii_uppercase();
    US_STATE_ABBREVS
        .iter()
        .find(|(abbr, _)| *abbr == key)
        .map(|(_, full)| *full)
}

/// Case-insensitive equality on trimmed strings.
fn eq_ci(a: &str, b: &str) -> bool {
    a.trim().eq_ignore_ascii_case(b.trim())
}

/// Does `hit` satisfy the qualifier `q`? A qualifier may itself be comma-separated (e.g.
/// `"MD, USA"`); any one token matching is enough. A token matches a hit when it equals (case-
/// insensitively) the hit's `admin1`, the US-abbrev→full expansion of the token compared to
/// `admin1`, the `country`, or the `country_code` (§1.1 client-side disambiguation).
fn hit_matches_qualifier(hit: &GeoHit, q: &str) -> bool {
    q.split(',')
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .any(|token| {
            let admin1 = hit.admin1.as_deref();
            let country = hit.country.as_deref();
            let country_code = hit.country_code.as_deref();

            admin1.is_some_and(|a| eq_ci(token, a))
                || expand_us_state(token).is_some_and(|full| admin1.is_some_and(|a| eq_ci(full, a)))
                || country.is_some_and(|c| eq_ci(token, c))
                || country_code.is_some_and(|cc| eq_ci(token, cc))
        })
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
/// `admin` is the optional qualifier from a `"City, ST"` input ([`parse_place_query`]): when
/// `Some`, the winner is the **top-by-population hit that matches the qualifier** (overriding the
/// global top), so `"Springfield, IL"` resolves to Illinois even though Springfield, MO is more
/// populous. A qualifier that matches no hit is non-fatal: we fall back to the global
/// top-by-population and add an explanatory note. `None` preserves the original behavior exactly.
///
/// The §3.3 pick/ambiguity/not-found tests pin it.
pub fn resolve_geocoded(
    hits: &[GeoHit],
    query_name: &str,
    admin: Option<&str>,
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

    // Pick the winner: with a qualifier, the top-by-population hit that matches it (if any);
    // otherwise the global top. A non-matching qualifier is recorded as a fallback note below.
    let mut notes = Notes::new();
    let top = match admin {
        Some(q) => match ranked.iter().find(|h| hit_matches_qualifier(h, q)) {
            Some(matched) => *matched,
            None => {
                let global_top = ranked[0];
                notes.push(format!(
                    "qualifier \"{q}\" didn't match any result; using {}, {}",
                    global_top.name,
                    global_top.admin1.as_deref().unwrap_or("")
                ));
                global_top
            }
        },
        None => ranked[0],
    };
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

    if hits.len() > 1 {
        // List the top alternatives (everything but the winner), population desc. Each is
        // `"{name}, {admin1}"` so a same-named hit is distinguishable by region.
        let alternatives: Vec<String> = ranked
            .iter()
            .filter(|h| !std::ptr::eq(**h, top))
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
