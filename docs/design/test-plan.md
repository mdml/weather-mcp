# Test plan — the Phase 2 coverage bar

> **Status: frozen-enough (Phase 1).** This is the verifier bar Phase 2 grinds against. It exists
> *before* the build on purpose: because [tool-specs](tool-specs.md) is frozen, the test
> checklist is **fully enumerable now** — "coverage" is defined as *every contract clause has a
> test*, not a gameable line-percentage. This is the concrete expansion of
> [DEVELOPMENT.md § Testing strategy](../guides/DEVELOPMENT.md#testing-strategy--mock-the-outermost-boundary-not-collaborators)
> for the three real tools. See also [0005](../decisions/0005-hands-off-agent-development.md) (verifiers over approval gates).

## The principle: tests derive from the spec

1. **Coverage = every clause of [tool-specs](tool-specs.md) is exercised by a named test.** The
   checklist in §3 *is* the definition. A request param, an output field, an error code, an edge
   rule (Feb 29, cross-year window, ERA5 clamp) — each maps to a test. This is what makes the
   bar knowable in advance.
2. **The un-mockable tests come first.** The MCP conformance test (real stdio session) and the
   live smoke test (real Open-Meteo) are written *before* the rest, so a suite of mock-only
   tests can't masquerade as coverage ([DEVELOPMENT.md](../guides/DEVELOPMENT.md#-just-check--the-verifier-stack-in-order)).
   The Phase 0 conformance harness already does this for `server_info`; Phase 2 extends it.
3. **Mock the outermost boundary, not collaborators** — see the test seam below.

## 1. The test seam — why no HTTP mock is needed

The architecture splits I/O from logic so the bulk of testing is **pure functions over recorded
JSON**, offline and deterministic:

```
HTTP  ──►  parse        ──►  aggregate (compare.rs)  ──►  assemble CallToolResult
(I/O)      (pure: &str→struct)  (pure: struct→struct)      (pure: struct→JSON)
```

- **`compare.rs` and the parse/assemble layers are pure** — they take parsed/recorded data and
  return values. Tested directly against fixtures. No network, no mock framework.
- **The Open-Meteo client sits behind a trait** (e.g. `WeatherData` with `forecast` / `archive`
  / `geocode`). A **fixture-backed impl** serves recorded JSON in tests, so even full tool
  handlers (and the conformance session) run **offline and deterministic**. The **real HTTP
  impl** is swapped in only for `just test-live`.
- The HTTP GET itself is the *only* thing not covered offline — and that's exactly what
  `test-live` exists for (catching upstream drift), kept thin and separate.

> This trait seam is the one small addition to [ARCHITECTURE.md](../guides/ARCHITECTURE.md)'s
> `openmeteo/` boundary that Phase 2 introduces. It's what lets "every output field appears in a
> snapshot" be true without hitting the network.

## 2. Fixtures (`tests/fixtures/`)

Recorded once from real Open-Meteo responses, then committed (the dir exists, empty, from
Phase 0). Minimum set:

| Fixture | Purpose |
|---|---|
| `forecast_<city>.json` | a real Forecast response → forecast parsing + `get_forecast` snapshot |
| `archive_<city>_<span>.json` | a multi-year ERA5 daily response → `get_historical` + the `compare_period` baseline & period (sliced client-side) |
| `geocode_<name>.json` | a Geocoding response with several hits → location resolution + ambiguity note |
| `geocode_empty.json` | zero results → `location_not_found` |
| `error_rate_limited.json` / HTTP 429 | → `upstream_rate_limited` |
| `error_upstream.json` (`{"error":true,"reason":…}`) | → `upstream_error` (reason passed through) |

Edge cases are covered by **slicing/crafting** from the above (a leap-year window, an
ERA5-lag-boundary end date), not by new network captures.

## 3. The coverage checklist — spec clause → test

Organized by layer, innermost (cheapest, highest-value) first.

### 3.1 `compare.rs` — pure aggregation (the crown jewel)
The differentiator's logic; aim for **full branch coverage** here (optionally enforced via
`cargo-llvm-cov` in CI).

- [ ] per-variable aggregation: precipitation/snow **sum**, temperature/wind **mean** ([§1.4](tool-specs.md#14-the-curated-variable-set))
- [ ] calendar-window extraction: same MM-DD window pulled from each baseline year ([§4.2](tool-specs.md#42-calendar-window-matching))
- [ ] cross-year window wrap (e.g. Dec→Feb spans `Y → Y+1`) ([§4.2](tool-specs.md#42-calendar-window-matching))
- [ ] Feb 29 included only in leap years; `notes` entry when relevant ([§4.2](tool-specs.md#42-calendar-window-matching))
- [ ] baseline distribution stats: `mean`, `stddev`, `min`, `max`, per-year `values[]` ([§4.4](tool-specs.md#44-output--envelope-16))
- [ ] anomaly: `absolute`, `percent`, `standardized` (z-score), `percentile_rank`, human `rank` ([§4.4](tool-specs.md#44-output--envelope-16))
- [ ] `include_series=true`: day-by-day `period` series + per-day-of-window `baseline_daily` mean/σ ([§4.5](tool-specs.md#45-optional-day-by-day-series-include_seriestrue))

### 3.2 `openmeteo/` — parsing & error mapping
- [ ] deserialize each fixture (forecast / archive / geocode) into typed structs
- [ ] unit mapping: `units` enum → the three Open-Meteo unit params; labels echoed correctly ([§1.2](tool-specs.md#12-units))
- [ ] WMO `weather_code` → label decode table ([Appendix B](tool-specs.md#appendix-b--wmo-weather-codes-decode-table))
- [ ] HTTP 429 → `upstream_rate_limited`; `{"error":true,"reason"}` / 5xx → `upstream_error` (reason in `message`); timeout → `upstream_unavailable` ([§1.5](tool-specs.md#15-error-model))

### 3.3 Location resolution ([§1.1](tool-specs.md#11-location--name-or-coordinates))
- [ ] `location` name → coords via geocode fixture; top-by-population pick; resolved place echoed ([§1.6](tool-specs.md#16-shared-output-envelope))
- [ ] multiple strong matches → chosen one + alternatives in `notes` (non-fatal)
- [ ] zero matches → `location_not_found`
- [ ] `lat`/`lon` passed directly → `source:"coordinates"`, name fields null
- [ ] neither, or both `location` and `lat`/`lon` → `invalid_request`

### 3.4 Tool handlers — `insta` snapshots (offline, fixture-backed client)
- [ ] `tools/list` schema snapshot for **all three** tools (request param schemas)
- [ ] `get_forecast` success snapshot — every output field present ([§2](tool-specs.md#2-get_forecast))
- [ ] `get_historical` success snapshot, incl. curated-variable columns ([§3](tool-specs.md#3-get_historical))
- [ ] `compare_period` success snapshot, incl. per-year array + (separately) `include_series` ([§4](tool-specs.md#4-compare_period--the-differentiator))
- [ ] one `is_error` result snapshot **per error code** ([§1.5](tool-specs.md#15-error-model))
- [ ] ERA5-lag clamp emits the `notes` entry, not an error ([§1.7](tool-specs.md#17-the-era5-lag-archive-tools))
- [ ] date guards: `start>end`, before 1940, future end → `invalid_date_range`

### 3.5 Conformance — real stdio session (extends `tests/conformance.rs`)
- [ ] `initialize` → `tools/list` lists exactly `[get_forecast, get_historical, compare_period]`
- [ ] `tools/call` each tool end-to-end against the fixture-backed client

### 3.6 Live smoke — `just test-live` (network, thin, separate)
- [ ] one real `forecast`, one real `archive`, one real `geocode` — assert shape parses
- [ ] **confirm `temperature_2m_mean` is available on the Archive API** (the one verify-at-build
  flag from [§1.4](tool-specs.md#14-the-curated-variable-set)); fall back to `(max+min)/2` if not

## 4. Done bar

Phase 2 is "tested" when:

- every checkbox in §3 is a passing, named test;
- every error `code` and every spec output field is exercised (§3.2–3.4);
- `compare.rs` hits full branch coverage;
- `just check` is green (fmt · clippy `-D warnings` · build · nextest incl. snapshots + conformance);
- `just test-live` passes against the real API.

## 5. Build order (test-first, for the Phase 2 fanout)

Write the bar before the impl so there's something to grind green:

1. **Seam + conformance skeleton** — the `WeatherData` trait, a fixture-backed impl, and the
   conformance test asserting the three tools (failing until they exist).
2. **`compare.rs` unit tests** against archive fixtures (§3.1) — then make them pass.
3. **Parsing + location + tool-handler snapshots** (§3.2–3.4) — then make them pass.
4. **Real HTTP impl + `test-live`** (§3.6) last — the only network-touching code, behind the
   now-proven seam.

This is the parallelizable shape: step 1 is the shared foundation (one agent), then one agent
per tool over the trait, each grinding its slice of §3 to green → PR.
