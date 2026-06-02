# Now

> The single pointer to current focus. Start here each session.

## State

**Phase 3 — the executable bar is green → Phase 4 next.** The Phase 0 harness landed
([#1](https://github.com/mdml/weather-mcp/pull/1)), the design specs are frozen
([docs/design/](../design/)), the Phase 2 red bar was authored, and now **Phase 3 has filled the
pure logic behind the `WeatherData` seam** — landed as five file-disjoint slices (parse/error-map,
dates+location, the `compare_period` aggregation, handler wiring + conformance snapshots, and the
real HTTP client + live smoke) via [#6](https://github.com/mdml/weather-mcp/pull/6)–[#10](https://github.com/mdml/weather-mcp/pull/10)
onto an integration branch that merged green in one shot ([#11](https://github.com/mdml/weather-mcp/pull/11)).
`just check` is **fully green** (39 passed; fmt/clippy/build/nextest incl. the four `insta`
conformance snapshots) and `just test-live` passes **3/3 against the real Open-Meteo API**. Next is
**Phase 4 — the MCP App views** ([0006](../decisions/0006-phased-delivery.md), [0005](../decisions/0005-hands-off-agent-development.md)).

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

## Next concrete step — Phase 4: the MCP App views

Phase 3 is complete: the three tools are implemented behind the seam, `just check` is green, and
the `tools/call` output shapes are pinned by the conformance snapshots — so the pure-data contract
the app consumes ([app-spec](../design/app-spec.md)) is now frozen *and exercised*.

Phase 4 builds the **MCP App** forecast + trend views against that frozen output. It is **gated on
an empirical check, resolved in-phase, not on paper**: does CCD render an MCP App inline? (see
[Decisions still open](#decisions-still-open) and
[roadmap.md § Open questions](roadmap.md#open-questions)). Settle that render check first, then
build the views.

## Decisions still open

Only the two **empirical** MCP-App-rendering checks remain, and they're verified in-phase, not
on paper (see [roadmap.md § Open questions](roadmap.md#open-questions)): does CCD render an MCP
App inline (Phase 4 gate), and does Claude mobile render MCP App UI resources at all (Phase 5
go/no-go). All Phase 1 design questions are resolved in the frozen specs.
