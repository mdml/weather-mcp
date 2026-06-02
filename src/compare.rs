//! `compare_period` aggregation — pure, deterministic, fixture-tested (the crown jewel, §4).
//!
//! Takes parsed archive data (the baseline years + the period of interest) and produces the
//! historical-trend comparison: per-variable period scalar, the baseline distribution (incl. the
//! per-year `values[]` that powers the trend chart), and the anomaly scores. Optionally the
//! day-by-day series (§4.5). No I/O — fully testable offline against fixtures, with expected
//! numbers from an *independent* oracle (test-plan §3.1).
//!
//! The §3.1 hand-asserted tests pin the [`compare`] entry point.

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
pub fn compare(
    baseline: &ArchiveData,
    period: &ArchiveData,
    spec: &CompareSpec,
) -> Result<CompareOutput, WeatherError> {
    let start_mmdd = mmdd(&spec.period_start);
    let end_mmdd = mmdd(&spec.period_end);
    // Cross-year iff the end MM-DD precedes the start MM-DD (window wraps the new year).
    let cross_year = end_mmdd < start_mmdd;
    let window = format!("{start_mmdd}..{end_mmdd}");

    let period_info = PeriodInfo {
        start: spec.period_start.clone(),
        end: spec.period_end.clone(),
        window,
    };

    let n_years = (spec.baseline_end_year - spec.baseline_start_year + 1) as usize;
    let baseline_info = BaselineInfo {
        reference: format!("{}-{}", spec.baseline_start_year, spec.baseline_end_year),
        start_year: spec.baseline_start_year,
        end_year: spec.baseline_end_year,
        n_years,
    };

    let mut comparisons = Vec::with_capacity(spec.variables.len());
    for &variable in &spec.variables {
        let column = variable.compare_column();
        let aggregation = variable.aggregation();
        let unit = variable.unit_label(spec.units);

        // period_value: aggregate the (already-windowed) period column over all of its rows.
        let period_col = period.daily.columns.get(column);
        let period_value = aggregate_indices(
            period_col,
            (0..period.daily.time.len())
                .collect::<Vec<_>>()
                .iter()
                .copied(),
            aggregation,
        );

        // Per baseline year: select the in-window rows and aggregate.
        let baseline_col = baseline.daily.columns.get(column);
        let mut year_values: Vec<BaselineYearValue> = Vec::with_capacity(n_years);
        for year in spec.baseline_start_year..=spec.baseline_end_year {
            let indices = baseline
                .daily
                .time
                .iter()
                .enumerate()
                .filter_map(|(i, date)| {
                    if in_window(date, year, &start_mmdd, &end_mmdd, cross_year) {
                        Some(i)
                    } else {
                        None
                    }
                });
            let value = aggregate_indices(baseline_col, indices, aggregation);
            year_values.push(BaselineYearValue { year, value });
        }

        let stats = baseline_stats(&year_values);
        let anomaly = anomaly(period_value, &stats, &year_values, variable);

        comparisons.push(Comparison {
            variable,
            unit,
            aggregation,
            period_value,
            baseline: stats,
            anomaly,
        });
    }

    let mut notes: Notes = Vec::new();
    if window_covers_feb29(&start_mmdd, &end_mmdd, cross_year)
        && baseline_has_mixed_leap_years(spec.baseline_start_year, spec.baseline_end_year)
    {
        notes.push("Feb 29 is included only in leap years; baseline day-counts vary.".to_string());
    }

    let series = if spec.include_series {
        Some(build_series(baseline, period, spec))
    } else {
        None
    };

    let payload = ComparePayload {
        period: period_info,
        baseline: baseline_info,
        comparisons,
        series,
    };

    Ok(CompareOutput { payload, notes })
}

// ---------------------------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------------------------

/// The `MM-DD` slice of a `YYYY-MM-DD` date string.
fn mmdd(date: &str) -> String {
    date[5..].to_string()
}

/// The year parsed from a `YYYY-MM-DD` date string.
fn year_of(date: &str) -> i32 {
    date[0..4].parse().unwrap_or(0)
}

/// Whether `date` falls in the MM-DD window *within* baseline year `year`, honoring wrap.
fn in_window(date: &str, year: i32, start_mmdd: &str, end_mmdd: &str, cross_year: bool) -> bool {
    let d_year = year_of(date);
    let d_mmdd = &date[5..];
    if cross_year {
        (d_year == year && d_mmdd >= start_mmdd) || (d_year == year + 1 && d_mmdd <= end_mmdd)
    } else {
        d_year == year && d_mmdd >= start_mmdd && d_mmdd <= end_mmdd
    }
}

/// Aggregate the `Some` values at `indices` of `col` per `aggregation`. Sum = total of present
/// values; Mean = sum / count of present values (`None` never counts toward the denominator).
fn aggregate_indices(
    col: Option<&Vec<Option<f64>>>,
    indices: impl Iterator<Item = usize>,
    aggregation: Aggregation,
) -> f64 {
    let Some(col) = col else { return 0.0 };
    let mut sum = 0.0;
    let mut count = 0usize;
    for i in indices {
        if let Some(Some(v)) = col.get(i) {
            sum += v;
            count += 1;
        }
    }
    match aggregation {
        Aggregation::Sum => sum,
        Aggregation::Mean => {
            if count == 0 {
                0.0
            } else {
                sum / count as f64
            }
        }
    }
}

/// Population mean + stddev over a slice of f64 (divide by n, not n-1).
fn mean_and_pop_stddev(values: &[f64]) -> (f64, f64) {
    let n = values.len();
    if n == 0 {
        return (0.0, 0.0);
    }
    let mean = values.iter().sum::<f64>() / n as f64;
    let var = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n as f64;
    (mean, var.sqrt())
}

/// The baseline distribution stats for one variable's per-year values.
fn baseline_stats(year_values: &[BaselineYearValue]) -> BaselineStats {
    let vals: Vec<f64> = year_values.iter().map(|v| v.value).collect();
    let (mean, stddev) = mean_and_pop_stddev(&vals);
    let min = vals.iter().copied().fold(f64::INFINITY, f64::min);
    let max = vals.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    BaselineStats {
        mean,
        stddev,
        min,
        max,
        values: year_values.to_vec(),
    }
}

/// The full anomaly score set for the period value vs the baseline distribution.
fn anomaly(
    period_value: f64,
    stats: &BaselineStats,
    year_values: &[BaselineYearValue],
    variable: Variable,
) -> Anomaly {
    let n_years = year_values.len();
    let absolute = period_value - stats.mean;
    let percent = if stats.mean == 0.0 {
        0.0
    } else {
        absolute / stats.mean * 100.0
    };
    let standardized = if stats.stddev == 0.0 {
        0.0
    } else {
        absolute / stats.stddev
    };
    let below = year_values
        .iter()
        .filter(|v| v.value < period_value)
        .count();
    let percentile_rank = if n_years == 0 {
        0.0
    } else {
        100.0 * below as f64 / n_years as f64
    };
    let greater = year_values
        .iter()
        .filter(|v| v.value > period_value)
        .count();
    let rank_pos = 1 + greater;
    let total = n_years + 1;
    let rank = format!(
        "{}-{} of {}",
        ordinal(rank_pos),
        descriptor(variable),
        total
    );

    Anomaly {
        absolute,
        percent,
        standardized,
        percentile_rank,
        rank,
    }
}

/// The per-variable rank descriptor (most-extreme-first naming).
fn descriptor(variable: Variable) -> &'static str {
    match variable {
        Variable::Precipitation => "wettest",
        Variable::Temperature => "warmest",
        Variable::Snowfall => "snowiest",
        Variable::Wind => "windiest",
    }
}

/// English ordinal for a 1-based position (1st, 2nd, 3rd, 4th, …, 11th/12th/13th, 21st, …).
fn ordinal(n: usize) -> String {
    let suffix = match (n % 10, n % 100) {
        (_, 11..=13) => "th",
        (1, _) => "st",
        (2, _) => "nd",
        (3, _) => "rd",
        _ => "th",
    };
    format!("{n}{suffix}")
}

/// Whether the window covers 02-29 (honoring wrap).
fn window_covers_feb29(start_mmdd: &str, end_mmdd: &str, cross_year: bool) -> bool {
    const FEB29: &str = "02-29";
    if cross_year {
        FEB29 >= start_mmdd || FEB29 <= end_mmdd
    } else {
        start_mmdd <= FEB29 && FEB29 <= end_mmdd
    }
}

/// Gregorian leap-year test.
fn is_leap(y: i32) -> bool {
    y % 4 == 0 && (y % 100 != 0 || y % 400 == 0)
}

/// Whether the baseline year range contains at least one leap and one non-leap year.
fn baseline_has_mixed_leap_years(start_year: i32, end_year: i32) -> bool {
    let mut has_leap = false;
    let mut has_non_leap = false;
    for y in start_year..=end_year {
        if is_leap(y) {
            has_leap = true;
        } else {
            has_non_leap = true;
        }
    }
    has_leap && has_non_leap
}

/// Build the optional day-by-day series (§4.5): the period's MM-DD axis, the period curve per
/// variable column, and the baseline per-day climatology (mean ± population stddev) aligned to it.
fn build_series(baseline: &ArchiveData, period: &ArchiveData, spec: &CompareSpec) -> Series {
    let window_days: Vec<String> = period
        .daily
        .time
        .iter()
        .map(|d| d[5..].to_string())
        .collect();

    let mut period_map: BTreeMap<String, Vec<Option<f64>>> = BTreeMap::new();
    let mut baseline_daily: BTreeMap<String, DailyClimatology> = BTreeMap::new();

    // Pre-index the baseline rows by MM-DD so each window day's climatology is a single lookup.
    for &variable in &spec.variables {
        let column = variable.compare_column();

        if let Some(col) = period.daily.columns.get(column) {
            period_map.insert(column.to_string(), col.clone());
        } else {
            period_map.insert(column.to_string(), vec![None; window_days.len()]);
        }

        let baseline_col = baseline.daily.columns.get(column);
        let mut means = Vec::with_capacity(window_days.len());
        let mut stddevs = Vec::with_capacity(window_days.len());
        for mmdd_key in &window_days {
            let day_values: Vec<f64> = baseline
                .daily
                .time
                .iter()
                .enumerate()
                .filter(|(_, date)| &date[5..] == mmdd_key.as_str())
                .filter_map(|(i, _)| baseline_col.and_then(|c| c.get(i).copied().flatten()))
                .collect();
            let (mean, stddev) = mean_and_pop_stddev(&day_values);
            means.push(mean);
            stddevs.push(stddev);
        }
        baseline_daily.insert(
            column.to_string(),
            DailyClimatology {
                mean: means,
                stddev: stddevs,
            },
        );
    }

    Series {
        window_days,
        period: period_map,
        baseline_daily,
    }
}
