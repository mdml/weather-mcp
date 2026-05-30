# 0005 — Hands-off agent development: verifiers over approval gates

**Date:** 2026-05-30
**Status:** active

## Context

The goal is to develop this **almost entirely hands-off with agents** as the primary mode. The
premise that makes that safe: **favor verifiers over approval gates.** Every "is this right?"
should be answerable by a command, not by a human reading a diff.

This project is close to best-case for that — and the reason is Rust, not in spite of it
([0002](0002-build-in-rust-with-rmcp.md)): the compiler + clippy + borrow checker collapse the
"plausible vs. correct" gap that normally forces human babysitting. The spec is narrow and
external (a documented API in a documented protocol), and the I/O boundary is cleanly mockable.

## Decision

Build the project around a **verifier stack** the agent grinds against, with **CI as the real
merge gate** — and **build the harness before the weather logic.**

### The verifier stack — one `just check`

`just check` runs, in order:

- `cargo fmt --check`
- `cargo clippy -- -D warnings`
- `cargo build`
- `cargo nextest run`
- `insta` snapshot tests on tool-output shapes
- **the MCP conformance test** — spawns the server over stdio and scripts a real client
  session (`initialize` → `tools/list` → `tools/call`). This is the verifier agents can't game
  with mocks.

Agent-facing recipes so calls auto-approve: `just check`, `just test`, `just test-live`,
`just mcp-smoke`, `just run`. Agents work in worktrees; commit attribution uses an `agent(...)`
conventional-commit type so the log self-documents who did what.

### CI as the real gate

Public repo → free GitHub Actions. On every push: `fmt` / `clippy` / `test` / `build`, plus
`cargo-deny` + `cargo-audit` for the supply-chain story. The human reviews a green checkmark,
not every line. Docker image + Fly.io preview per branch ([0006](0006-phased-delivery.md)).

### The boundary / mock rule

Mock the **outermost boundary, not collaborators**: record real Open-Meteo responses as
fixtures; run parsing/aggregation against them offline + deterministic; keep a thin live-API
layer (`just test-live`) to catch upstream drift.

## Why this ordering (harness first)

- Once the scaffold is green, the three real tools are just the agent grinding against a bar
  already defined — human time is front-loaded (define the bar) and intermittent (taste calls
  on naming / error messages / output shapes), not continuous babysitting.
- It's the cheap test of the skepticism: if the harness comes together clean, the
  hands-off bet holds; if it fights us, that's learned for one session's cost, not by
  committing blind.

## The one real trap, and the mitigation

Agents writing tests that **pass but don't exercise real behavior** (the mock problem). The
mitigation is **structural, not vigilance**: build the **live smoke test + MCP conformance
test first**, so the rest of the test suite can't be satisfied by mocking away the real work.

## Consequences

- Session 1 (this work + the next) targets **green CI on a skeleton** — one trivial tool, the
  full `just check` stack wired, fixtures + one passing conformance test, Actions CI, a Fly.io
  preview — *before* any weather logic.
- Requires toolchain: `cargo-nextest`, `cargo-deny`, `cargo-audit`, `insta`, `just`.
- Deploy touchpoints (Fly.io first-run, later OAuth) remain human-in-loop; Phase 2 local stdio
  is fully agent-able.

## Alternatives considered

- **Approval-gate every diff (human reads everything)** — rejected: doesn't scale to hands-off
  development; the whole point is that a verifier, not a human read, answers "is this right?"
- **Features first, harness later** — rejected: inverts the leverage; you discover the harness
  fights you only after sinking effort into logic. Harness-first is the cheap skepticism test.

## See also

- [0002 — Build in Rust with rmcp](0002-build-in-rust-with-rmcp.md)
- [0006 — Phased delivery](0006-phased-delivery.md)
- [Development / verifier bar](../guides/DEVELOPMENT.md)
