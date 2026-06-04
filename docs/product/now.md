# Now

> The single pointer to current focus. Start here each session.

## State

**Phase 3 — the executable bar is green → Phase 3.5 (precip data source) next.** The Phase 0 harness landed
([#1](https://github.com/mdml/weather-mcp/pull/1)), the design specs are frozen
([docs/design/](../design/)), the Phase 2 red bar was authored, and now **Phase 3 has filled the
pure logic behind the `WeatherData` seam** — landed as five file-disjoint slices (parse/error-map,
dates+location, the `compare_period` aggregation, handler wiring + conformance snapshots, and the
real HTTP client + live smoke) via [#6](https://github.com/mdml/weather-mcp/pull/6)–[#10](https://github.com/mdml/weather-mcp/pull/10)
onto an integration branch that merged green in one shot ([#11](https://github.com/mdml/weather-mcp/pull/11)).
`just check` is **fully green** (39 passed; fmt/clippy/build/nextest incl. the four `insta`
conformance snapshots) and `just test-live` passes **3/3 against the real Open-Meteo API**.
Post-Phase-3 live dogfooding then landed two follow-up fixes — `"City, ST"` geocoding
([#14](https://github.com/mdml/weather-mcp/pull/14)) and inlined/flat tool schemas for MCP-client
compatibility ([#15](https://github.com/mdml/weather-mcp/pull/15)) — and surfaced an **ERA5
precipitation-accuracy problem**, so next is **Phase 3.5 — a precip data-source investigation**,
then **Phase 4 — the MCP App views**
([0006](../decisions/0006-phased-delivery.md), [0005](../decisions/0005-hands-off-agent-development.md)).

What exists now:

- **Code:** single crate `weather-mcp` (lib + bin) on rmcp 1.7 — stdio server exposing the three
  **fully-implemented** tools (`get_forecast`/`get_historical`/`compare_period`) over the
  `WeatherData` seam (fixture-backed for the deterministic tests, real reqwest/rustls `HttpClient`
  for live); `justfile` verifier stack (`check`/`test`/`test-live`/`mcp-smoke`/`run`), MCP
  conformance + `insta` snapshot tests (the pinned `tools/call` contract), GitHub Actions CI
  (`just check` + `cargo-deny`/`cargo-audit`).
- **Design specs (frozen):** [tool-specs](../design/tool-specs.md) (the 3-tool contract) +
  [app-spec](../design/app-spec.md) (the Phase 4 forecast + trend views and the output shapes
  they need) + [test-plan](../design/test-plan.md) (the Phase 2 coverage bar, enumerable now
  because the spec is frozen)
- Decision records: [docs/decisions/](../decisions/) (0001–0007)
- [Roadmap](roadmap.md) with the phased plan + open questions
- Guides: [ARCHITECTURE](../guides/ARCHITECTURE.md) · [DEVELOPMENT](../guides/DEVELOPMENT.md)
- Agent config: `.claude/settings.json`, `.codex/`, `.mcp.json`, [AGENTS.md](../../AGENTS.md)
- Secrets via **dotenvx** ([0007](../decisions/0007-secrets-via-dotenvx.md)): `GH_TOKEN` in
  `.env.local` (encrypted, gitignored), consumed per-command via `dotenvx run -f .env.local -- …`

**Deferred Phase-0 follow-ups** (cheap, do when needed): the Docker/Fly preview deploy and the
lefthook/commitlint + dotenvx `just` recipes.

## Next concrete step — Phase 3.5: precipitation data-source investigation

A live `compare_period` dogfood exposed a data-quality problem: **ERA5 precipitation (Open-Meteo)
diverges sharply from station gauges.** Baltimore's last 12 months read **98% of normal** where the
gauge record (Capital Weather Gang / BWI) shows a **72%-of-normal drought** — the tool is *correct*;
the *source* is wrong for point/recent/drought precip (ERA5 precip is model-derived + grid-averaged
~28 km; the finer ERA5-Land lags months behind → nulls for recent dates; Open-Meteo serves no gauge
product). ERA5 stays fine for temperature and the multi-decade long view.

**Phase 3.5 is research, not code:** evaluate gauge-based precip sources — NOAA **ACIS** (what CWG
uses; US-only, keyless), **NLDAS**, and a global gauge/satellite blend (**IMERG**/**CHIRPS**) to keep
global coverage — verify accuracy vs CWG for a few cities, and produce a recommendation + scope + an
[ADR-0001](../decisions/0001-data-source-open-meteo.md) amendment **before** any code. Then **Phase 4
— the MCP App views** builds on a precip source we trust (gated on the CCD inline-render check; see
[Decisions still open](#decisions-still-open)).

## Decisions still open

Only the two **empirical** MCP-App-rendering checks remain, and they're verified in-phase, not
on paper (see [roadmap.md § Open questions](roadmap.md#open-questions)): does CCD render an MCP
App inline (Phase 4 gate), and does Claude mobile render MCP App UI resources at all (Phase 5
go/no-go). All Phase 1 design questions are resolved in the frozen specs.
