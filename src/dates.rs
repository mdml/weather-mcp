//! Date-range validation (§1.5 `invalid_date_range`) and the ERA5-lag clamp (§1.7).
//!
//! Pure and deterministic (today is passed in, not read from the clock), so it's unit-tested
//! offline. The §3.4 date-guard tests and the §1.7 clamp test pin it.

use crate::types::WeatherError;

/// The earliest date ERA5 covers ([0001](../../docs/decisions/0001-data-source-open-meteo.md)).
pub const ERA5_EPOCH: &str = "1940-01-01";

/// Validate a historical/compare window (§1.5): `start <= end`, `start >= 1940-01-01`, and `end`
/// not after `today` (`YYYY-MM-DD`). Any violation is `invalid_date_range`.
///
/// The §3.4 tests pin it.
pub fn validate_date_range(start: &str, end: &str, today: &str) -> Result<(), WeatherError> {
    // All inputs are `YYYY-MM-DD`, so a lexicographic compare is a chronological compare.
    if start > end {
        return Err(WeatherError::invalid_date_range(format!(
            "start ({start}) is after end ({end})"
        )));
    }
    if start < ERA5_EPOCH {
        return Err(WeatherError::invalid_date_range(format!(
            "start ({start}) is before the ERA5 epoch ({ERA5_EPOCH})"
        )));
    }
    if end > today {
        return Err(WeatherError::invalid_date_range(format!(
            "end ({end}) is in the future (after today, {today})"
        )));
    }
    Ok(())
}

/// Clamp `requested_end` down to `last_available` (the ERA5 ~5-day-lag boundary) when it is later,
/// returning the effective end plus a human `notes` string when a clamp happened (§1.7). Never
/// errors and never silently shortens without a note.
///
/// The §1.7 clamp test pins it.
pub fn clamp_end_to_archive(requested_end: &str, last_available: &str) -> (String, Option<String>) {
    if requested_end > last_available {
        let note = format!("end clamped from {requested_end} to {last_available} (ERA5 5-day lag)");
        (last_available.to_string(), Some(note))
    } else {
        (requested_end.to_string(), None)
    }
}
