//! Date-range validation (§1.5 `invalid_date_range`) and the ERA5-lag clamp (§1.7).
//!
//! Pure and deterministic (today is passed in, not read from the clock), so it's unit-tested
//! offline. Stubbed in Phase 2; the §3.4 date-guard tests and the §1.7 clamp test are the bar.

use crate::types::WeatherError;

/// The earliest date ERA5 covers ([0001](../../docs/decisions/0001-data-source-open-meteo.md)).
pub const ERA5_EPOCH: &str = "1940-01-01";

/// Validate a historical/compare window (§1.5): `start <= end`, `start >= 1940-01-01`, and `end`
/// not after `today` (`YYYY-MM-DD`). Any violation is `invalid_date_range`.
///
/// Phase 3 fills this in; the §3.4 tests pin it.
pub fn validate_date_range(_start: &str, _end: &str, _today: &str) -> Result<(), WeatherError> {
    todo!("Phase 3: start<=end, start>=1940-01-01, end<=today (test-plan §3.4)")
}

/// Clamp `requested_end` down to `last_available` (the ERA5 ~5-day-lag boundary) when it is later,
/// returning the effective end plus a human `notes` string when a clamp happened (§1.7). Never
/// errors and never silently shortens without a note.
///
/// Phase 3 fills this in; the §1.7 clamp test pins it.
pub fn clamp_end_to_archive(
    _requested_end: &str,
    _last_available: &str,
) -> (String, Option<String>) {
    todo!("Phase 3: clamp + 'end clamped from X to Y (ERA5 5-day lag)' note (test-plan §1.7)")
}
