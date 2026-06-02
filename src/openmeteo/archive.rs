//! Archive (ERA5) API: wire types, the parsed daily block, and the parse/slice seam.
//!
//! One wide archive request feeds both `get_historical` and the `compare_period` baseline +
//! period — the parsed [`ArchiveData`] is sliced client-side per window ([`slice_archive`], §4.6).
//! Columns are kept as a name→values map so a variable's columns and the slicing logic stay
//! generic. `parse_archive`/`slice_archive` are stubbed in Phase 2; the §3.1/§3.2 tests pin them.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::types::WeatherError;

/// The parsed daily archive block. `columns` maps an Open-Meteo daily column name (e.g.
/// `precipitation_sum`) to its values, index-aligned with `time`. A `BTreeMap` keeps column
/// order deterministic for snapshots. Serializes to Open-Meteo's `{time:[], <col>:[]}` shape.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ArchiveDaily {
    pub time: Vec<String>,
    #[serde(flatten)]
    pub columns: BTreeMap<String, Vec<Option<f64>>>,
}

/// Parsed Archive response. Lat/lon/elevation/timezone ride along for the envelope (§1.1).
#[derive(Debug, Clone, PartialEq)]
pub struct ArchiveData {
    pub latitude: f64,
    pub longitude: f64,
    pub elevation: f64,
    pub timezone: String,
    pub daily: ArchiveDaily,
}

// ---------------------------------------------------------------------------------------------
// Wire types
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct RawArchive {
    pub latitude: f64,
    pub longitude: f64,
    pub elevation: f64,
    pub timezone: String,
    pub daily: RawArchiveDaily,
}

/// Raw daily block: `time` plus every requested column captured into the flattened map.
#[derive(Debug, Clone, Deserialize)]
pub struct RawArchiveDaily {
    pub time: Vec<String>,
    #[serde(flatten)]
    pub columns: BTreeMap<String, Vec<Option<f64>>>,
}

/// Parse an Archive API body into [`ArchiveData`] (deserialize [`RawArchive`], reshape).
///
/// Phase 3 fills this in; the §3.2 deserialize test pins it.
pub fn parse_archive(_body: &str) -> Result<ArchiveData, WeatherError> {
    todo!("Phase 3: RawArchive -> ArchiveData (test-plan §3.2)")
}

/// Slice a parsed [`ArchiveData`] to the inclusive `[start, end]` `YYYY-MM-DD` window, preserving
/// index alignment across `time` and every column. Used by the fixture client to emulate a
/// windowed upstream request, and by `compare_period` to carve the baseline/period out of one
/// wide fetch (§4.6).
///
/// Phase 3 fills this in; the §3.1 calendar-window tests pin it.
pub fn slice_archive(_data: &ArchiveData, _start: &str, _end: &str) -> ArchiveData {
    todo!("Phase 3: inclusive date-window slice preserving alignment (test-plan §3.1)")
}
