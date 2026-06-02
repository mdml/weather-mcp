# Now

> The single pointer to current focus. Start here each session.

## State

**Phase 2 — the executable bar is authored (red) → Phase 3 next.** The Phase 0 harness landed
([#1](https://github.com/mdml/weather-mcp/pull/1)), the design specs are frozen
([docs/design/](../design/)), and now the **Phase 2 red bar is in review**: the fixtures, the
`WeatherData` seam + full type surface (logic stubbed `todo!()`), and the hand-asserted tests
that encode the specs. `just check` is **green at fmt/clippy/build and red at exactly the
unimplemented logic** — the reviewable bar. Next is **Phase 3 — the hands-off red→green grind**
([0006](../decisions/0006-phased-delivery.md), [0005](../decisions/0005-hands-off-agent-development.md)).

What exists now:

- **Code:** single crate `weather-mcp` (lib + bin) on rmcp 1.7 — stdio server exposing the three
  real tools (`get_forecast`/`get_historical`/`compare_period`) whose **logic is stubbed**
  (`todo!()`) behind the `WeatherData` seam; `justfile` verifier stack
  (`check`/`test`/`test-live`/`mcp-smoke`/`run`), MCP conformance + `insta` snapshot tests,
  GitHub Actions CI (`just check` + `cargo-deny`/`cargo-audit`).
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

## Next concrete step — Phase 3: make the bar green (hands-off grind)

The Phase 2 bar is authored and under review (the branch `agent/phase-2-test-bar`). It delivered:

1. **Fixtures** recorded into `tests/fixtures/` (Boston forecast / wide 1991–2026 archive /
   geocode + empty) plus a [`RECORDING.md`](../../tests/fixtures/RECORDING.md) capture recipe; the
   `temperature_2m_mean` verify-at-build flag is **resolved** (present on Archive).
2. **Type surface + the `WeatherData` seam** as a `lib` (`types`/`wmo`/`location`/`dates`/`compare`/
   `openmeteo`), with a real fixture-backed client and the pure logic stubbed `todo!()`.
3. **The hand-asserted tests** encoding the specs — the `compare.rs` oracle (numbers from an
   independent `jq` calc), parsing + error-mapping, location, date guards, and the conformance
   session asserting the three tools (`tools/list` green; `tools/call` red until Phase 3).

Phase 3 fills the pure logic behind the seam until `just check` is green and the `insta`
snapshots (the three `tools/call` results) are generated + accepted. The grind is parallelizable:
one slice per module — `openmeteo` parse/error-mapping + `dates`, `location`, `compare`, then the
handler wiring in `server.rs` + the real `HttpClient`/`test-live` last
([0006](../decisions/0006-phased-delivery.md)). Each `todo!()` names its test-plan clause.

## Decisions still open

Only the two **empirical** MCP-App-rendering checks remain, and they're verified in-phase, not
on paper (see [roadmap.md § Open questions](roadmap.md#open-questions)): does CCD render an MCP
App inline (Phase 4 gate), and does Claude mobile render MCP App UI resources at all (Phase 5
go/no-go). All Phase 1 design questions are resolved in the frozen specs.

> The stale source-comment phase numbering from the Phase 0 skeleton is **resolved**: this PR
> rewrote those stub files, so comments now reference Phase 2 (the bar) / Phase 3 (the grind).
