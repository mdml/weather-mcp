# 0004 — Minimal three-tool surface

**Date:** 2026-05-30
**Status:** active

## Context

The best-maintained reference server (`cmer81/open-meteo-mcp`) mirrors essentially the entire
Open-Meteo surface — forecast, archive, air quality, marine, elevation, geocoding, ensemble,
flood, seasonal, climate projections, plus model-pinned variants — roughly 30 tools. That's a
lot of surface to expose, secure, and maintain, and most of it is irrelevant to the actual
goal.

## Decision

Keep the surface deliberately small — **three tools** that do exactly the job:

- **`get_forecast(lat, lon)`** → current conditions + N-day forecast.
- **`get_historical(lat, lon, start, end, vars)`** → daily history from the ERA5 archive API.
- **`compare_period(lat, lon, this_year_range, baseline_years)`** → the "this year vs. past
  decade" aggregation that is the actual point of the project.

Crib parameter names and shapes from `cmer81/open-meteo-mcp` (and the lean
`gbrigandi/mcp-server-openmeteo` 4-tool surface) rather than inventing them.

## Why small

- **Does exactly the comparison I want** rather than making me eyeball raw numbers.
  `compare_period` is the tool that justifies the whole project — it's not in any existing
  server.
- **Smaller attack surface, less to maintain.** Open-Meteo needs no auth, so there's no key
  management to amortize across many tools — keeping it lean is pure upside.
- **Sharper for the model.** Three well-named tools are easier for Claude to choose between
  than thirty.

## Why not the full surface

- The 30-tool kitchen sink (air quality, marine, flood, ensemble, climate projections, etc.)
  adds surface, maintenance, and ambiguity with no payoff for the forecast + trend use case.
  Additional tools can be added later if a real need appears.

## Consequences

- Several parameters are **open questions** to settle before/while implementing
  ([roadmap](../product/roadmap.md)):
  - **`vars`** — which variables beyond precipitation + temperature (snow, wind, humidity)?
  - **`compare_period` baseline** — trailing N years, or a 30-year climate normal? Which
    summary stats (mean, anomaly, percentile rank)?
  - **Location handling** — pass `lat`/`lon` every call, or add a place→coords resolver / saved
    home location?
- Tool output shapes are pinned by `insta` snapshot tests; the MCP conformance test exercises
  `tools/list` + `tools/call` against the real surface ([0005](0005-hands-off-agent-development.md)).

## Alternatives considered

- **Mirror the full Open-Meteo surface (~30 tools)** — rejected as over-broad for the goal.
- **Two tools (forecast + historical), compute comparisons in-conversation** — rejected:
  `compare_period` is the differentiator and belongs server-side so the aggregation is
  deterministic and testable, not re-derived by the model each time.

## See also

- [0001 — Data source: Open-Meteo](0001-data-source-open-meteo.md)
- [0002 — Build in Rust with rmcp](0002-build-in-rust-with-rmcp.md)
- [0006 — Phased delivery](0006-phased-delivery.md)
