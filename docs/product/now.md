# Now

> The single pointer to current focus. Start here each session.

## State

**Phase 1 — Design.** The Phase 0 harness has landed: a skeleton `rmcp` stdio server with the
full verifier stack and green CI on `main` ([#1](https://github.com/mdml/weather-mcp/pull/1)).
Next is **design before build** — spec the tool interfaces and the future MCP-app UX so Phase 2
is a clean build against frozen specs ([0006](../decisions/0006-phased-delivery.md)).

What exists now:

- **Code:** single crate `weather-mcp` on rmcp 1.7 — stdio server with one trivial `server_info`
  tool, `justfile` verifier stack (`check`/`test`/`test-live`/`mcp-smoke`/`run`), MCP conformance
  + `insta` snapshot tests, GitHub Actions CI (`just check` + `cargo-deny`/`cargo-audit`).
- Decision records: [docs/decisions/](../decisions/) (0001–0007)
- [Roadmap](roadmap.md) with the phased plan + open questions
- Guides: [ARCHITECTURE](../guides/ARCHITECTURE.md) · [DEVELOPMENT](../guides/DEVELOPMENT.md)
- Agent config: `.claude/settings.json`, `.codex/`, `.mcp.json`, [AGENTS.md](../../AGENTS.md)
- Secrets via **dotenvx** ([0007](../decisions/0007-secrets-via-dotenvx.md)): `GH_TOKEN` in
  `.env.local` (encrypted, gitignored), consumed per-command via `dotenvx run -f .env.local -- …`

**Deferred Phase-0 follow-ups** (cheap, do when needed): the Docker/Fly preview deploy and the
lefthook/commitlint + dotenvx `just` recipes.

## Next concrete step — Phase 1 design (the design session)

Human-led design, no fanout. Produce design files (under `docs/design/`) that freeze the
contracts Phase 2 builds against:

1. **Tool interface specs** for `get_forecast`, `get_historical`, `compare_period`
   ([0004](../decisions/0004-minimal-tool-surface.md)) — request params, output JSON shapes,
   units, error model. Crib parameter shapes from `cmer81/open-meteo-mcp` and the Open-Meteo
   Forecast + Archive API docs.
2. **MCP-app specs** for the future trend-chart / anomaly view (Phase 3) — what it renders and
   which tool outputs feed it, designed now so the Phase 2 outputs are app-ready.
3. **Resolve the [open questions](roadmap.md#open-questions)** as part of the specs: the `vars`
   set, `compare_period` baseline + stats, default-location handling, archive
   rate-limits/caching.

Then: **Phase 2 — build the three tools** against the frozen specs. The parallel build fanout
returns here — one agent per tool, grinding `just check` green → PR.

## Decisions still open

See [roadmap.md § Open questions](roadmap.md#open-questions) — resolved in Phase 1 design, plus
the two MCP-App-rendering verifications gating Phases 3/4.

> Note: a few source-comment phase references in the merged skeleton still say the old numbering
> (e.g. "Phase 1" for the tools, "Phase 3" for HTTP); those get corrected in the first Phase 2
> code PR, which rewrites those stub files anyway.
