# Now

> The single pointer to current focus. Start here each session.

## State

**Phase 1 — Design (complete) → Phase 2 next.** The Phase 0 harness landed
([#1](https://github.com/mdml/weather-mcp/pull/1)) and the **design specs are now frozen**: the
three tool contracts and the future MCP-app UX are pinned in [docs/design/](../design/), with the
roadmap's open questions resolved. Next is **Phase 2 — build the three tools** against those
frozen specs ([0006](../decisions/0006-phased-delivery.md)).

What exists now:

- **Code:** single crate `weather-mcp` on rmcp 1.7 — stdio server with one trivial `server_info`
  tool, `justfile` verifier stack (`check`/`test`/`test-live`/`mcp-smoke`/`run`), MCP conformance
  + `insta` snapshot tests, GitHub Actions CI (`just check` + `cargo-deny`/`cargo-audit`).
- **Design specs (frozen):** [tool-specs](../design/tool-specs.md) (the 3-tool contract) +
  [app-spec](../design/app-spec.md) (the Phase 3 forecast + trend views and the output shapes
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

## Next concrete step — Phase 2 build (the fanout)

Specs are frozen ([tool-specs](../design/tool-specs.md), [app-spec](../design/app-spec.md)), so
Phase 2 is a clean agent-grind against a known bar. Build the three real tools against the
Forecast + ERA5 Archive APIs:

1. **`get_forecast`** — current conditions + N-day daily forecast ([tool-specs §2](../design/tool-specs.md#2-get_forecast)).
2. **`get_historical`** — daily ERA5 record for a window ([tool-specs §3](../design/tool-specs.md#3-get_historical)).
3. **`compare_period`** — the differentiator: climate-normal anomaly aggregation, pure +
   fixture-tested in `compare.rs` ([tool-specs §4](../design/tool-specs.md#4-compare_period--the-differentiator)).

Plus the shared client/scaffolding: the `openmeteo/` client (forecast + archive + geocoding)
behind a fixture-testable trait seam, the location/units/error conventions
([tool-specs §1](../design/tool-specs.md#1-shared-conventions)), fixtures, and snapshots that pin
the frozen output shapes. The build is **test-first** — the [test-plan](../design/test-plan.md)
defines the coverage bar (every spec clause → a test) and the sequencing: seam + conformance
skeleton first, then `compare.rs` unit tests, then tool snapshots, then the real HTTP client +
`test-live`. That makes the fanout clean: one agent builds the shared seam, then one agent per
tool grinds its slice of the checklist `just check`-green → PR.

## Decisions still open

Only the two **empirical** MCP-App-rendering checks remain, and they're verified in-phase, not
on paper (see [roadmap.md § Open questions](roadmap.md#open-questions)): does CCD render an MCP
App inline (Phase 3 gate), and does Claude mobile render MCP App UI resources at all (Phase 4
go/no-go). All Phase 1 design questions are resolved in the frozen specs.

> Note: a few source-comment phase references in the merged skeleton still say the old numbering
> (e.g. "Phase 1" for the tools, "Phase 3" for HTTP); those get corrected in the first Phase 2
> code PR, which rewrites those stub files anyway.
