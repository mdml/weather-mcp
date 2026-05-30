//! Open-Meteo client — the outermost I/O boundary (ARCHITECTURE.md).
//!
//! Phase 1 fills this in with the Forecast API (`forecast.rs`) and the historical ERA5
//! Archive API (`archive.rs`). It is the only layer the live smoke test (`just
//! test-live`) exercises against the real API. Empty in Phase 0.
