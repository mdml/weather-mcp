# 0002 — Build it ourselves, in Rust, on the `rmcp` SDK

**Date:** 2026-05-30
**Status:** active

## Context

Surveyed the ~8 existing Open-Meteo MCP servers on GitHub. Most (isdaniel, JeremyMorgan,
fcto-demos, nprousalidis) are forecast-only tutorials with no archive API — useless for the
trend question ([0001](0001-data-source-open-meteo.md)). Two cover history:

- **`cmer81/open-meteo-mcp`** (npm `open-meteo-mcp-server`) — the best-maintained: TypeScript,
  Node ≥22, v1.6.1 (2026-03-20), 22 releases, 51★, active CI. Mirrors the full Open-Meteo
  surface (~30 tools incl. ERA5 archive, air quality, marine, flood, climate projections,
  model-pinned variants).
- **`gbrigandi/mcp-server-openmeteo`** — Rust, ships pre-built binaries, a clean 4-tool surface
  (current / forecast / historical / geocode). But v0.1.0, 3 commits, 1★, unmaintained.

So the realistic options are: adopt cmer81 as a dependency, fork the Rust skeleton, or build
fresh.

## Decision

**Build it ourselves, in Rust**, on the official Rust MCP SDK:

- **SDK: `rmcp`** — the official Rust SDK (`modelcontextprotocol/rust-sdk`), on `tokio`.
  Pin **`rmcp` 1.7.0** (latest stable, released 2026-05-13). Note the 1.x macro API
  (`#[tool_router]` / `#[tool]` / `#[tool_handler]`, `serve(stdio())`) differs from 0.x.
- **References, not dependencies:** crib tool shapes and parameter names from
  `cmer81/open-meteo-mcp`; use `gbrigandi/mcp-server-openmeteo`'s lean 4-tool surface as a
  skeleton reference for the shape we want.

## Why Rust + build-fresh

- **The compiler is a brutal, free verifier.** What makes agent work need babysitting is that
  "plausible" and "correct" diverge silently. Rust collapses that gap — `cargo build` + `clippy
  -D warnings` + the borrow checker give a failure signal rich and immediate enough for an
  agent to self-correct in a loop. Rust is harder for humans for the same reason it's easier
  for an agent harness. This is the foundation of [0005](0005-hands-off-agent-development.md).
- **The scope suits Rust.** A small, self-contained, no-auth API wrapper is a natural fit —
  little surface to fight the borrow checker over, and the result is a single static binary
  that's trivial to ship and deploy.
- **The spec is narrow and external.** Wrap a documented API (Open-Meteo) in a documented
  protocol (MCP), three tools already defined, with a reference impl to crib from. Low
  ambiguity → high agent success.

## Why not adopt cmer81 as a dependency

- It's TypeScript (not Rust) and a ~30-tool kitchen sink. We
  want a deliberately small surface ([0004](0004-minimal-tool-surface.md)). Cribbing its
  well-considered tool/parameter shapes gives us the benefit without the dependency or bloat.

## Why not fork the Rust skeleton

- `gbrigandi` is v0.1.0 / 3 commits / unmaintained — not adoptable. Its tool surface is a
  useful shape reference, but starting fresh on current `rmcp` 1.x is cleaner than rehabbing a
  stale 0.x-era skeleton.

## Consequences

- We own the full implementation and its maintenance — acceptable given the narrow scope.
- Must track `rmcp` releases; pin the version and bump deliberately.
- `cargo-deny` + `cargo-audit` in CI cover the supply-chain story
  ([0005](0005-hands-off-agent-development.md)).

## Alternatives considered

- **Adopt `cmer81/open-meteo-mcp`** — rejected (TS, too large; use as reference).
- **Fork `gbrigandi/mcp-server-openmeteo`** — rejected (stale; use as shape reference).
- **Forecast-only tutorial servers** — rejected (no archive API; can't answer the trend
  question).

## See also

- [0001 — Data source: Open-Meteo](0001-data-source-open-meteo.md)
- [0004 — Minimal tool surface](0004-minimal-tool-surface.md)
- [0005 — Hands-off agent development](0005-hands-off-agent-development.md)
