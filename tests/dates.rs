//! Date guards + ERA5-lag clamp (test-plan §3.4 date guards, §1.7).

use weather_mcp::dates::{clamp_end_to_archive, validate_date_range};
use weather_mcp::types::ErrorCode;

const TODAY: &str = "2026-06-02";

#[test]
fn valid_window_passes() {
    validate_date_range("2000-01-01", "2000-12-31", TODAY).expect("valid window");
}

#[test]
fn start_after_end_is_invalid() {
    let err = validate_date_range("2001-01-01", "2000-01-01", TODAY).expect_err("start>end");
    assert_eq!(err.code, ErrorCode::InvalidDateRange);
}

#[test]
fn start_before_1940_is_invalid() {
    let err = validate_date_range("1939-12-31", "1945-01-01", TODAY).expect_err("pre-1940");
    assert_eq!(err.code, ErrorCode::InvalidDateRange);
}

#[test]
fn future_end_is_invalid() {
    let err = validate_date_range("2026-01-01", "2030-01-01", TODAY).expect_err("future end");
    assert_eq!(err.code, ErrorCode::InvalidDateRange);
}

#[test]
fn end_clamped_when_past_era5_lag_with_note() {
    let (end, note) = clamp_end_to_archive("2026-05-30", "2026-05-25");
    assert_eq!(end, "2026-05-25");
    let note = note.expect("a clamp note");
    assert!(
        note.contains("ERA5"),
        "note should mention ERA5, got {note:?}"
    );
}

#[test]
fn no_clamp_when_within_lag() {
    let (end, note) = clamp_end_to_archive("2026-05-20", "2026-05-25");
    assert_eq!(end, "2026-05-20");
    assert!(note.is_none());
}
