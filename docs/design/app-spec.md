# App spec — Phase 4 views (forecast + trend)

> **Status: frozen-enough (Phase 1).** These views are *built* in Phase 4, but they're
> **specified now** for one reason: so the Phase 3 tool outputs are already shaped to feed them
> without a rewrite. The contract this doc pins is the **data contract** (which fields each view
> reads); the visual details can still move, the data shapes should not.
> See [tool-specs](tool-specs.md) · [roadmap](../product/roadmap.md) · [0006](../decisions/0006-phased-delivery.md).

## Two views, two jobs

| View | Job | Fed by | Frequency |
|---|---|---|---|
| **A. Forecast** | the everyday "what's it like today / next 10 days" | [`get_forecast`](tool-specs.md#2-get_forecast) | **the common case** |
| **B. Trend / anomaly** | "is this period unusual vs. the climate baseline" | [`compare_period`](tool-specs.md#4-compare_period--the-differentiator) | the differentiator |

The forecast view is what people reach for most — a polished, glanceable forecast is table
stakes, and the bar is set by apps like Apple Weather. The trend view is the thing no consumer
app does. **Both are Phase 4**, both gated on the same render check below; neither requires a
Phase 3 tool-output change (validated in each view's data contract).

**Rendering gate (do not skip):** before building either view, deploy a *trivial* MCP App and
confirm **CCD renders MCP App UI inline** ([0006 gating criteria](../decisions/0006-phased-delivery.md)).
If the host only surfaces tool JSON, the views are wasted effort.

---

## A. Forecast view — the everyday case

Mirrors the familiar Apple-Weather-style layout: a current-conditions header over a vertical
N-day list. The reference (an Apple Weather 10-day list) — each row: day · condition icon ·
low temp · a temperature **range bar** · high temp, with a precip-probability % on wet days.

### Panels

- **Current card** — today's big number + condition, feels-like, hi/lo, and a couple of detail
  chips (humidity, wind).
- **N-day list** — one row per forecast day: weekday label, weather icon + label, low, the
  range bar, high, and precip-probability when non-trivial.

### The range bar (pure client-side)

Each row's bar spans that day's `[min, max]` positioned **within the week's overall
`[min, max]`**, tinted by absolute temperature (cool→warm gradient). Today's row marks the
*current* temperature as a dot. All of this is computed in the view from the daily arrays + the
current temp — **no extra tool data**.

### Data contract (what the tools must emit — the contract already requires it)

| Element | Reads | From `get_forecast` |
|---|---|---|
| Current card | `temperature`, `apparent_temperature`, `relative_humidity`, `weather_code`/`weather`, `wind_speed`, `is_day` | `current` |
| Row: day | `time[i]` | `daily.time` |
| Row: icon + label | `weather_code[i]` (+ decoded `weather`, [Appendix B](tool-specs.md#appendix-b--wmo-weather-codes-decode-table)) | `daily.weather_code` |
| Row: low / high | `temperature_min[i]` / `temperature_max[i]` | `daily.temperature_min`, `daily.temperature_max` |
| Row: precip % | `precipitation_probability_max[i]` | `daily.precipitation_probability_max` |
| Range bar | the daily min/max arrays + `current.temperature` | `daily` + `current` |

**Validation:** every field above is already in the `get_forecast` contract
([tool-specs §2](tool-specs.md#2-get_forecast)) — shaping the output before building the UI is
exactly what Phase 1 is for. The only optional add we might want later is
`apparent_temperature_max/min` for a "feels like" range; flagged, not required for v1.

---

## B. Trend / anomaly view — the differentiator

Turns a `compare_period` result into the answer to *"is this period unusual, and by how much?"*
— the question raw JSON makes a human squint at.

### Panels (driven by one `compare_period` payload)

**B1. Anomaly headline (stat card)** — the one-glance verdict, per variable:

> **Precipitation — Jan 1 → May 25, 2026**
> **412 mm · +15% vs the 1991–2020 normal**
> 6th-wettest of 31 years · 81st percentile · +0.9σ

Reads `comparisons[].period_value`, `.anomaly.{percent,standardized,percentile_rank,rank}`,
`baseline.reference`, `period.window`.

**B2. Distribution / trend chart** — the per-baseline-year values across an x-axis of years,
with the baseline **mean line** and a **±1σ band**, and the current `period_value` drawn as a
highlighted marker sitting inside/outside the band. Widen the baseline (e.g. 1950→2025) and this
*is* the multi-decade trend explorer ([tool-specs §4.1](tool-specs.md#41-baseline--a-reference-window-default-the-climate-normal)).
Reads `baseline.values[]`, `baseline.{mean,stddev,min,max}`, `period_value`.

**B3. Within-period time series (optional, `include_series=true`)** — the period's day-by-day
curve over the MM-DD window against the baseline's per-day climatology mean ± σ band — shows
*when* the anomaly accrued. Reads `series.window_days`, `series.period.<col>`,
`series.baseline_daily.<col>.{mean,stddev}`.

### Data contract

| Panel | Reads | From |
|---|---|---|
| B1 | `period_value`, `anomaly.*`, `unit`, `aggregation` | `comparisons[]` |
| B1/B2 | `baseline.{reference,mean,stddev,min,max}`, `baseline.values[].{year,value}` | `comparisons[].baseline` |
| all | `period.window`, `location.{name,timezone}`, `units` | envelope |
| B3 | `series.*` | `compare_period(include_series=true)` |

Numbers arrive **pre-aggregated and pre-scored** — the view does layout, never statistics. The
math stays in `compare.rs` (deterministic, fixture-tested), out of untested view code. This is
the design-now reason `compare_period` returns the per-year `baseline.values` array and the
opt-in `series` block, not just a headline number.

---

## Shared — interactions, non-goals, build

**Interactions (Phase 4 detail, not frozen):** variable toggle; baseline toggle (re-call with
`1951`/`1980` for the pre-warming contrast); hover a year/day for its value + rank.

**Non-goals / deferred:** maps, multi-location overlays, animation; editing the query from
inside the view (the model re-calls the tool); mobile-specific layout — gated on the Phase 5
question of whether **Claude mobile renders MCP App UI resources at all**
([0006](../decisions/0006-phased-delivery.md)).

**Build (Phase 4):** Node/Vite HTML bundle served by the Rust server, via the `create-mcp-app`
skill + `@modelcontextprotocol/ext-apps`. A distinct web-frontend workstream from the Rust
server — it depends only on the data contracts above, so it can be built and iterated
independently once Phase 3 ships those shapes.
