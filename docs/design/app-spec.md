# App spec — the trend / anomaly view (Phase 3)

> **Status: frozen-enough (Phase 1).** This view is *built* in Phase 3, but it is **specified
> now** for one reason: so the Phase 2 tool outputs are already shaped to feed it without a
> rewrite. The contract this doc cares about is the **data contract** (§3) — the fields
> [`compare_period`](tool-specs.md#4-compare_period--the-differentiator) and
> [`get_historical`](tool-specs.md#3-get_historical) must emit. The visual details (§2) can
> still move; the data shape should not.
> See [roadmap](../product/roadmap.md) · [0006](../decisions/0006-phased-delivery.md).

## Why design it now

[0006](../decisions/0006-phased-delivery.md) defers the UI to Phase 3, but the cheapest way to
guarantee "Phase 2 outputs are app-ready" is to know what the app reads. The two design-now
consequences already baked into [tool-specs](tool-specs.md):

1. `compare_period` returns the **per-year `baseline.values` array**, not just summary stats —
   so the app can draw the *distribution*, not only the headline.
2. `compare_period` has an opt-in **`include_series`** block — so the day-by-day view is one
   call, not a second round-trip.

If those two weren't in the Phase 2 contract, Phase 3 would force a tool-output change. They
are, so it won't.

## 1. What it's for

One MCP-App resource, rendered inline by the host, that turns a `compare_period` result into the
answer to *"is this period unusual, and by how much?"* — the question raw JSON makes a human
squint at. It's the visual payoff of the differentiator tool.

**Rendering gate (do not skip):** before building this, deploy a *trivial* MCP App and confirm
**CCD renders MCP App UI inline** ([0006 gating criteria](../decisions/0006-phased-delivery.md)).
The whole view is wasted effort if the host only surfaces tool JSON.

## 2. The three panels

Driven entirely by a single `compare_period` payload (with `include_series=true` for panel C).

### A. Anomaly headline (stat card)
The one-glance verdict, per variable:

> **Precipitation — Jan 1 → May 25, 2026**
> **412 mm · +15% vs the 1991–2020 normal**
> 6th-wettest of 31 years · 81st percentile · +0.9σ

Source fields: `comparisons[].period_value`, `.anomaly.{percent,standardized,percentile_rank,
rank}`, `baseline.reference`, `period.window`. Color/sign from the anomaly (wetter/drier,
warmer/cooler).

### B. Distribution / trend chart
The per-baseline-year values as a bar or dot-strip across the x-axis of years, with the baseline
**mean line** and a **±1σ band**, and the current `period_value` drawn as a highlighted marker
that visibly sits inside/outside the band. This is what makes "unusual" legible.

Source fields: `baseline.values[] {year,value}`, `baseline.{mean,stddev,min,max}`,
`period_value`.

### C. Within-period time series (optional, `include_series=true`)
The period's day-by-day curve over the MM-DD window against the baseline's per-day climatology
mean ± σ band — shows *when* the anomaly accrued (e.g. "a wet April did it").

Source fields: `series.window_days`, `series.period.<col>`,
`series.baseline_daily.<col>.{mean,stddev}`.

## 3. Data contract (the part that's actually frozen)

The app consumes **only** these fields; Phase 2 must emit them with these shapes/units:

| Panel | Reads | From |
|---|---|---|
| A | `period_value`, `anomaly.{absolute,percent,standardized,percentile_rank,rank}`, `unit`, `aggregation` | `comparisons[]` |
| A/B | `baseline.{reference,mean,stddev,min,max}`, `baseline.values[].{year,value}` | `comparisons[].baseline` |
| B/A | `period.window`, `location.{name,timezone}`, `units` | envelope |
| C | `series.window_days`, `series.period`, `series.baseline_daily` | `compare_period(include_series=true)` |

Numbers arrive **pre-aggregated and pre-scored** — the app does no statistics, only layout. That
keeps the math in `compare.rs` (deterministic, fixture-tested) and out of untested view code.

## 4. Interactions (Phase 3 detail, not frozen)

- Variable toggle (a panel set per requested variable).
- Baseline toggle (re-call with `1951`/`1980` to contrast normal vs pre-warming reference).
- Hover a year in panel B → its value + how it ranks.

## 5. Non-goals / deferred

- Maps, multi-location overlays, animation.
- Editing the query from inside the view (the model re-calls the tool).
- Mobile-specific layout — gated on the Phase 4 question of whether **Claude mobile renders MCP
  App UI resources at all** ([0006](../decisions/0006-phased-delivery.md)).

## 6. Build notes (Phase 3)

Node/Vite HTML bundle served by the Rust server, via the `create-mcp-app` skill +
`@modelcontextprotocol/ext-apps` ([roadmap](../product/roadmap.md#phases)). A distinct
web-frontend workstream from the Rust server — it only depends on the §3 data contract, so it
can be built and iterated independently once Phase 2 ships that shape.
