//! `compare_period` aggregation — pure, deterministic, fixture-tested (the crown jewel, §4).
//!
//! Takes parsed archive data (the baseline years + the period of interest) and produces the
//! historical-trend comparison: per-variable period scalar, the baseline distribution (incl. the
//! per-year `values[]` that powers the trend chart), and the anomaly scores. Optionally the
//! day-by-day series (§4.5). No I/O — fully testable offline against fixtures, with expected
//! numbers from an *independent* oracle (test-plan §3.1).
//!
//! The [`compare`] entry point is stubbed in Phase 2; the §3.1 hand-asserted tests are the bar.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::openmeteo::archive::ArchiveData;
use crate::types::{Aggregation, Notes, Units, Variable, WeatherError};

// ---------------------------------------------------------------------------------------------
// Output payload (§4.4 / §4.5)
// ---------------------------------------------------------------------------------------------

/// The resolved period window (§4.4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PeriodInfo {
    pub start: String,
    pub end: String,
    /// `MM-DD..MM-DD` calendar window (§4.2).
    pub window: String,
}

/// The baseline reference window (§4.1/§4.4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct BaselineInfo {
    /// e.g. `"1991-2020"`.
    pub reference: String,
    pub start_year: i32,
    pub end_year: i32,
    pub n_years: usize,
}

/// One baseline year's aggregated value — the per-year array that turns the comparison into a
/// multi-decade trend explorer (§4.1/§4.4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct BaselineYearValue {
    pub year: i32,
    pub value: f64,
}

/// The baseline distribution for one variable (§4.4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct BaselineStats {
    pub mean: f64,
    pub stddev: f64,
    pub min: f64,
    pub max: f64,
    pub values: Vec<BaselineYearValue>,
}

/// How the period scored against the baseline distribution (§4.4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Anomaly {
    /// `period_value − baseline.mean`.
    pub absolute: f64,
    /// Relative to `baseline.mean`, percent.
    pub percent: f64,
    /// z-score: `(period_value − mean) / stddev`.
    pub standardized: f64,
    /// Percentile of `period_value` within the baseline distribution.
    pub percentile_rank: f64,
    /// Human rank, e.g. `"6th-wettest of 31"`.
    pub rank: String,
}

/// One variable's full comparison entry (§4.4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Comparison {
    pub variable: Variable,
    pub unit: String,
    pub aggregation: Aggregation,
    pub period_value: f64,
    pub baseline: BaselineStats,
    pub anomaly: Anomaly,
}

/// Per-day-of-window climatology: baseline mean ± σ for each MM-DD in the window (§4.5).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct DailyClimatology {
    pub mean: Vec<f64>,
    pub stddev: Vec<f64>,
}

/// The optional day-by-day series for the within-period time-series view (§4.5).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Series {
    /// The MM-DD axis.
    pub window_days: Vec<String>,
    /// This period, per day, keyed by Open-Meteo column.
    pub period: BTreeMap<String, Vec<Option<f64>>>,
    /// The baseline per-day climatology, keyed by Open-Meteo column.
    pub baseline_daily: BTreeMap<String, DailyClimatology>,
}

/// The full `compare_period` payload (§4.4), minus the shared envelope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ComparePayload {
    pub period: PeriodInfo,
    pub baseline: BaselineInfo,
    pub comparisons: Vec<Comparison>,
    /// Populated only when `include_series == true` (§4.5).
    pub series: Option<Series>,
}

// ---------------------------------------------------------------------------------------------
// Aggregation entry point
// ---------------------------------------------------------------------------------------------

/// Everything the aggregation needs beyond the archive data itself.
#[derive(Debug, Clone, PartialEq)]
pub struct CompareSpec {
    /// The period of interest, `YYYY-MM-DD` (already ERA5-clamped, §1.7).
    pub period_start: String,
    pub period_end: String,
    pub variables: Vec<Variable>,
    pub baseline_start_year: i32,
    pub baseline_end_year: i32,
    pub include_series: bool,
    pub units: Units,
}

/// The aggregation result: the serialized [`ComparePayload`] plus any non-fatal `notes` (e.g. the
/// Feb-29 leap-day case, §4.2) for the handler to merge into the shared envelope (§1.6).
#[derive(Debug, Clone, PartialEq)]
pub struct CompareOutput {
    pub payload: ComparePayload,
    pub notes: Notes,
}

/// Aggregate the `period` against the calendar-matched `baseline` window of each year in the
/// baseline range, producing the per-variable comparison (§4.2–§4.5). Pure and deterministic.
///
/// `baseline` covers the baseline years; `period` covers the period of interest (both sliced from
/// the wide archive fetch, §4.6). Statistical conventions (pinned by the §3.1 oracle tests and
/// recorded in tool-specs §4.4): **population** standard deviation; `percentile_rank` = share of
/// baseline years strictly below the period value; `rank` = the period's position among the
/// baseline years **plus itself**, ordered most-extreme-first (largest value first), with a
/// variable-specific descriptor (wettest / warmest / snowiest / windiest).
///
/// Phase 3 fills this in; the §3.1 oracle tests are the bar.
pub fn compare(
    _baseline: &ArchiveData,
    _period: &ArchiveData,
    _spec: &CompareSpec,
) -> Result<CompareOutput, WeatherError> {
    todo!("Phase 3: calendar-window aggregation + distribution stats + anomaly (test-plan §3.1)")
}
