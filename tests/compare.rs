//! `compare_period` aggregation — the crown jewel (test-plan §3.1).
//!
//! Hand-asserted against an **independent oracle**: the expected numbers were computed with `jq`
//! over the same fixture (a throwaway calc, not the impl), and the inputs are built by
//! [`common`] without going through the production parse/slice — keeping the oracle independent
//! of the code under test.
//!
//! Conventions pinned here (and recorded in tool-specs §4.4): population stddev; `percentile_rank`
//! = % of baseline years strictly below the period; `rank` = position among baseline + period,
//! largest-first, with a per-variable descriptor.

mod common;

use common::{approx, archive_fixture, crafted, slice};
use weather_mcp::compare::{compare, CompareSpec};
use weather_mcp::types::{Aggregation, Units, Variable};

fn boston_spec(variables: Vec<Variable>, include_series: bool) -> CompareSpec {
    CompareSpec {
        period_start: "2026-01-01".to_string(),
        period_end: "2026-05-25".to_string(),
        variables,
        baseline_start_year: 1991,
        baseline_end_year: 2020,
        include_series,
        units: Units::Metric,
    }
}

// ---- §3.1 precipitation: period SUM + distribution + anomaly, vs the jq oracle ----------------

#[test]
fn precipitation_sum_matches_oracle() {
    let wide = archive_fixture();
    let baseline = slice(&wide, "1991-01-01", "2020-12-31");
    let period = slice(&wide, "2026-01-01", "2026-05-25");
    let spec = boston_spec(vec![Variable::Precipitation], false);

    let out = compare(&baseline, &period, &spec).expect("compare succeeds");
    let c = out
        .payload
        .comparisons
        .iter()
        .find(|c| c.variable == Variable::Precipitation)
        .expect("precipitation comparison present");

    assert_eq!(c.aggregation, Aggregation::Sum);
    assert_eq!(c.unit, "mm");
    approx(c.period_value, 378.8, 1e-3);

    // Baseline distribution (1991–2020).
    approx(c.baseline.mean, 463.466_666_7, 1e-3);
    approx(c.baseline.stddev, 88.509_925_4, 1e-3); // population stddev
    approx(c.baseline.min, 306.2, 1e-3);
    approx(c.baseline.max, 657.8, 1e-3);
    assert_eq!(c.baseline.values.len(), 30);
    assert_eq!(c.baseline.values[0].year, 1991);
    approx(c.baseline.values[0].value, 359.4, 1e-3);
    let v2010 = c.baseline.values.iter().find(|v| v.year == 2010).unwrap();
    approx(v2010.value, 657.8, 1e-3);

    // Anomaly (period was a dry one — negative).
    approx(c.anomaly.absolute, -84.666_666_7, 1e-3);
    approx(c.anomaly.percent, -18.268_124_3, 1e-3);
    approx(c.anomaly.standardized, -0.956_578_2, 1e-4);
    approx(c.anomaly.percentile_rank, 16.666_666_7, 1e-3);
    assert_eq!(c.anomaly.rank, "26th-wettest of 31");

    // Envelope-adjacent payload metadata.
    assert_eq!(out.payload.period.window, "01-01..05-25");
    assert_eq!(out.payload.period.start, "2026-01-01");
    assert_eq!(out.payload.period.end, "2026-05-25");
    assert_eq!(out.payload.baseline.reference, "1991-2020");
    assert_eq!(out.payload.baseline.n_years, 30);
}

// ---- §3.1 temperature: period MEAN path -------------------------------------------------------

#[test]
fn temperature_mean_matches_oracle() {
    let wide = archive_fixture();
    let baseline = slice(&wide, "1991-01-01", "2020-12-31");
    let period = slice(&wide, "2026-01-01", "2026-05-25");
    let spec = boston_spec(vec![Variable::Temperature], false);

    let out = compare(&baseline, &period, &spec).expect("compare succeeds");
    let c = out
        .payload
        .comparisons
        .iter()
        .find(|c| c.variable == Variable::Temperature)
        .expect("temperature comparison present");

    assert_eq!(c.aggregation, Aggregation::Mean);
    assert_eq!(c.unit, "°C");
    approx(c.period_value, 3.759_310_3, 1e-4);
    approx(c.baseline.mean, 3.528_131_9, 1e-4);
    approx(c.baseline.stddev, 1.239_204_4, 1e-4);
    approx(c.baseline.min, 0.927_586_2, 1e-4);
    approx(c.baseline.max, 6.173_287_7, 1e-4);
    assert_eq!(c.baseline.values.len(), 30);
    approx(c.anomaly.absolute, 0.231_178_4, 1e-4);
    approx(c.anomaly.standardized, 0.186_553_9, 1e-4);
    approx(c.anomaly.percentile_rank, 56.666_666_7, 1e-3);
    assert_eq!(c.anomaly.rank, "14th-warmest of 31");
}

// ---- §3.1 two variables in one call -----------------------------------------------------------

#[test]
fn both_default_variables_returned_in_order() {
    let wide = archive_fixture();
    let baseline = slice(&wide, "1991-01-01", "2020-12-31");
    let period = slice(&wide, "2026-01-01", "2026-05-25");
    let spec = boston_spec(vec![Variable::Temperature, Variable::Precipitation], false);

    let out = compare(&baseline, &period, &spec).expect("compare succeeds");
    let vars: Vec<Variable> = out.payload.comparisons.iter().map(|c| c.variable).collect();
    assert_eq!(vars, vec![Variable::Temperature, Variable::Precipitation]);
}

// ---- §3.1 include_series: day-by-day curve + per-day climatology ------------------------------

#[test]
fn include_series_adds_window_curve_and_climatology() {
    let wide = archive_fixture();
    let baseline = slice(&wide, "1991-01-01", "2020-12-31");
    let period = slice(&wide, "2026-01-01", "2026-05-25");
    let spec = boston_spec(vec![Variable::Precipitation], true);

    let out = compare(&baseline, &period, &spec).expect("compare succeeds");
    let series = out
        .payload
        .series
        .expect("series present when include_series=true");

    assert_eq!(series.window_days.len(), 145); // 01-01..05-25, non-leap 2026
    assert_eq!(series.window_days.first().unwrap(), "01-01");
    assert_eq!(series.window_days.last().unwrap(), "05-25");

    let period_precip = &series.period["precipitation_sum"];
    assert_eq!(period_precip.len(), 145);
    assert_eq!(period_precip[0], Some(2.20)); // 2026-01-01

    let clim = &series.baseline_daily["precipitation_sum"];
    assert_eq!(clim.mean.len(), 145);
    assert_eq!(clim.stddev.len(), 145);
    approx(clim.mean[0], 2.476_666_7, 1e-4); // mean precip on 01-01 across 1991–2020
}

// ---- §3.1 cross-year window wrap (Dec → Feb spans Y → Y+1) ------------------------------------

#[test]
fn cross_year_window_wraps_into_next_year() {
    // Window 12-30 .. 02-15 (end MM-DD precedes start MM-DD) ⇒ wraps. For baseline year Y the
    // window spans Dec Y → Feb Y+1. Decoys at 02-16 and 06-01 must be excluded.
    let baseline = crafted(
        "precipitation_sum",
        &[
            ("2000-12-30", 1.0), // year-2000 window: 1+2+3 = 6
            ("2001-01-10", 2.0),
            ("2001-02-15", 3.0),
            ("2001-02-16", 99.0), // excluded (after window end)
            ("2001-06-01", 99.0), // excluded (outside)
            ("2001-12-30", 4.0),  // year-2001 window: 4+5+6 = 15
            ("2002-01-10", 5.0),
            ("2002-02-15", 6.0),
        ],
    );
    let period = crafted("precipitation_sum", &[("2002-12-31", 12.0)]);
    let spec = CompareSpec {
        period_start: "2002-12-30".to_string(),
        period_end: "2003-02-15".to_string(),
        variables: vec![Variable::Precipitation],
        baseline_start_year: 2000,
        baseline_end_year: 2001,
        include_series: false,
        units: Units::Metric,
    };

    let out = compare(&baseline, &period, &spec).expect("compare succeeds");
    let c = &out.payload.comparisons[0];
    approx(c.period_value, 12.0, 1e-9);
    let y2000 = c.baseline.values.iter().find(|v| v.year == 2000).unwrap();
    let y2001 = c.baseline.values.iter().find(|v| v.year == 2001).unwrap();
    approx(y2000.value, 6.0, 1e-9);
    approx(y2001.value, 15.0, 1e-9);
    approx(c.baseline.mean, 10.5, 1e-9);
    approx(c.baseline.stddev, 4.5, 1e-9); // population stddev of {6, 15}
    approx(c.anomaly.percentile_rank, 50.0, 1e-9); // one of two baseline years below 12
    assert_eq!(c.anomaly.rank, "2nd-wettest of 3");
}

// ---- §3.1 Feb 29 included only in leap years, with a note -------------------------------------

#[test]
fn feb_29_included_in_leap_years_only_and_noted() {
    // Window 02-28 .. 03-01. 2000 is a leap year (has 02-29); 2001 is not.
    let baseline = crafted(
        "precipitation_sum",
        &[
            ("2000-02-28", 1.0),
            ("2000-02-29", 1.0), // leap day — included for 2000 ⇒ sum 3
            ("2000-03-01", 1.0),
            ("2001-02-28", 1.0),
            ("2001-03-01", 1.0), // no 02-29 ⇒ sum 2
        ],
    );
    let period = crafted("precipitation_sum", &[("2026-02-28", 5.0)]);
    let spec = CompareSpec {
        period_start: "2026-02-28".to_string(),
        period_end: "2026-03-01".to_string(),
        variables: vec![Variable::Precipitation],
        baseline_start_year: 2000,
        baseline_end_year: 2001,
        include_series: false,
        units: Units::Metric,
    };

    let out = compare(&baseline, &period, &spec).expect("compare succeeds");
    let c = &out.payload.comparisons[0];
    let y2000 = c.baseline.values.iter().find(|v| v.year == 2000).unwrap();
    let y2001 = c.baseline.values.iter().find(|v| v.year == 2001).unwrap();
    approx(y2000.value, 3.0, 1e-9); // leap year includes 02-29
    approx(y2001.value, 2.0, 1e-9); // non-leap year does not
    assert!(
        out.notes
            .iter()
            .any(|n| n.contains("Feb 29") || n.contains("29")),
        "expected a leap-day note, got {:?}",
        out.notes
    );
}
