# Now

> The single pointer to current focus. Start here each session.

## State

**Phase 0 — Harness.** The planning docs and agent config are in place (this repo's first
session). There is **no Rust code yet** — by design ([0005](../decisions/0005-hands-off-agent-development.md):
build the harness before the weather logic).

What exists now:

- Decision records: [docs/decisions/](../decisions/) (0001–0006)
- [Roadmap](roadmap.md) with the phased plan + open questions
- Guides: [ARCHITECTURE](../guides/ARCHITECTURE.md) (planned layout) · [DEVELOPMENT](../guides/DEVELOPMENT.md) (the verifier bar)
- Agent config: `.claude/settings.json`, `.codex/`, `.mcp.json`, [AGENTS.md](../../AGENTS.md)
- `README.md`, `LICENSE` (Apache-2.0)

## Next concrete step — scaffold the skeleton (the build session)

Target: **green CI on a skeleton.** No real weather logic.

1. `cargo` project + `rust-toolchain.toml` (stable, with `rustfmt` + `clippy`); pin `rmcp`
   1.7.0 + `tokio`. Single small crate (see [ARCHITECTURE](../guides/ARCHITECTURE.md)).
2. Skeleton `rmcp` stdio server with **one trivial tool** (e.g. `ping` / `server_info`) using
   the 1.x `#[tool_router]` / `#[tool]` / `#[tool_handler]` macros + `serve(stdio())`.
3. The `justfile` verifier stack: `check`, `test`, `test-live`, `mcp-smoke`, `run`
   (spec in [DEVELOPMENT](../guides/DEVELOPMENT.md)).
4. Fixtures dir + **one passing MCP conformance test** (spawn over stdio; `initialize` →
   `tools/list` → `tools/call`).
5. GitHub Actions CI: `fmt` / `clippy -D warnings` / `nextest` / `build` + `cargo-deny` +
   `cargo-audit`.
6. `Dockerfile` + `fly.toml` scaffold (deploy is human-in-loop; CI builds the image).
7. `.gitignore`; optional lefthook + commitlint (conventional commits incl. `agent` type).

Then: **Phase 1 proper** — the three real tools ([0004](../decisions/0004-minimal-tool-surface.md))
against the Forecast + Archive APIs, cribbing shapes from `cmer81/open-meteo-mcp`.

## Decisions still open

See [roadmap.md § Open questions](roadmap.md#open-questions) — `vars` set, `compare_period`
baseline + stats, location handling, archive rate-limits/caching, and the two
MCP-App-rendering verifications gating Phase 2/3.
