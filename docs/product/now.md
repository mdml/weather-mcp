# Now

> The single pointer to current focus. Start here each session.

## State

**Phase 1 — Design (complete) → Phase 2 next.** The Phase 0 harness landed
([#1](https://github.com/mdml/weather-mcp/pull/1)) and the **design specs are now frozen**: the
three tool contracts, the app UX, and the test plan are pinned in [docs/design/](../design/), with
the roadmap's open questions resolved. Next is **Phase 2 — set the test harness + executable
bar** against those specs, so Phase 3 (the implementation) is a hands-off red→green grind
([0006](../decisions/0006-phased-delivery.md), [0005](../decisions/0005-hands-off-agent-development.md)).

What exists now:

- **Code:** single crate `weather-mcp` on rmcp 1.7 — stdio server with one trivial `server_info`
  tool, `justfile` verifier stack (`check`/`test`/`test-live`/`mcp-smoke`/`run`), MCP conformance
  + `insta` snapshot tests, GitHub Actions CI (`just check` + `cargo-deny`/`cargo-audit`).
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

## Next concrete step — Phase 2: set the executable bar

Write the verifier bar against the frozen specs *before* implementing, so Phase 3 is a clean
red→green grind. Per the [test-plan](../design/test-plan.md), Phase 2 delivers:

1. **Fixtures** — record real Open-Meteo forecast / archive / geocode responses (+ crafted error
   & edge fixtures) into `tests/fixtures/` ([test-plan §2](../design/test-plan.md#2-fixtures-testsfixtures)).
2. **Type surface + the `WeatherData` trait seam** as stubs (`todo!()`) — enough for tests to
   *compile* ([test-plan §1](../design/test-plan.md#1-the-test-seam--why-no-http-mock-is-needed)).
3. **The hand-asserted tests** that encode the specs — above all the pure `compare.rs`
   aggregation, with expected numbers from an *independent* oracle (a throwaway calc over the same
   fixture, not the impl) — plus parsing, location, error-mapping; extend the MCP conformance
   test to assert the three tools; scaffold the `insta` snapshot functions
   ([test-plan §3](../design/test-plan.md#3-the-coverage-checklist--spec-clause--test)).

The deliverable is a **well-defined red `just check`** — the bar a human reviews once (this is
where human attention concentrates). Then **Phase 3** is the hands-off fanout: one agent builds
the shared seam, one per tool fills in the pure logic until `just check` is green and the
snapshots are accepted → PR.

## Decisions still open

Only the two **empirical** MCP-App-rendering checks remain, and they're verified in-phase, not
on paper (see [roadmap.md § Open questions](roadmap.md#open-questions)): does CCD render an MCP
App inline (Phase 4 gate), and does Claude mobile render MCP App UI resources at all (Phase 5
go/no-go). All Phase 1 design questions are resolved in the frozen specs.

> Note: source-comment phase references in the merged skeleton still use the old numbering
> (e.g. "Phase 1" for the tools, "Phase 3" for HTTP) and predate this split; they get corrected
> in the first Phase 2 code PR, which rewrites those stub files anyway.
