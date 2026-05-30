# 0003 — Standalone public repo, Apache-2.0, hosted on Fly.io

**Date:** 2026-05-30
**Status:** active

## Context

This is a self-contained tool that wraps a public, keyless API — it carries no private content
and has no dependency on any larger codebase. That makes the choice of where it lives, how it's
licensed, and how it's hosted straightforward to settle up front. The candidate homes were:
fold it into an existing private repository, or give it its own standalone repository.

## Decision

- **Its own standalone, public GitHub repository** — not folded into any larger or private
  codebase.
- **License: Apache-2.0** (matches the `rmcp` SDK's license; explicit patent grant).
- **Hosting: Fly.io** (deferred to Phase 4 — see [0006](0006-phased-delivery.md)).
- **CI: GitHub Actions** (free for public repos) as the real merge gate — see
  [0005](0005-hands-off-agent-development.md).

## Why a standalone public repo

- **It's just code — no private content.** Nothing here needs to be gated, so a public repo is
  the simplest fit and unlocks free CI.
- **Self-contained.** The tool has no ties to any other codebase, so a dedicated repo keeps its
  history, issues, and releases focused and uncoupled.
- **Reproducible for others.** A standalone repo + free Actions CI + a Fly.io deploy story is
  something anyone can clone and run.

## Why Apache-2.0

- Matches `rmcp` (Apache-2.0), avoiding any license-compatibility friction with the core
  dependency, and provides an explicit patent grant — a sensible default for a public tool.

## Why Fly.io (not a self-hosted box)

- A self-contained, reproducible deploy story for an open-source project. Phase 4's remote +
  OAuth server (the path to Claude mobile) is the real infra lift; a managed platform keeps that
  story portable and reproducible for anyone, rather than tied to bespoke infrastructure.

## Consequences

- Public from day one — no private content, and **no secret ever lands in version control**.
  Dev/CI secrets (e.g. `GH_TOKEN`) stay out of git, managed locally via dotenvx
  ([0007](0007-secrets-via-dotenvx.md)).
- Deployment (Fly.io first-run, later OAuth) is **human-in-loop**, not agent-automated; Phase 2
  (local stdio) is fully agent-able. See [0006](0006-phased-delivery.md).
- A `LICENSE` file (Apache-2.0) lives at the repo root.

## Alternatives considered

- **Fold it into an existing private repository** — rejected: would couple a public,
  self-contained tool to an unrelated codebase's privacy posture and tooling, and forfeit free
  CI and a clean clone-and-run story.
- **MIT / dual MIT-OR-Apache-2.0** — considered; chose single Apache-2.0 to match `rmcp` and
  keep one license file.

## See also

- [0005 — Hands-off agent development](0005-hands-off-agent-development.md)
- [0006 — Phased delivery](0006-phased-delivery.md)
- [0007 — Secrets via dotenvx](0007-secrets-via-dotenvx.md)
