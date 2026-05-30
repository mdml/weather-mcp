# 0006 — Phased delivery + transport abstraction

**Date:** 2026-05-30
**Status:** active

## Context

Visualization and remote/mobile access are tempting to chase early, but each is a different
kind of lift (a web-frontend workstream; a real infra + auth lift) and would stall the core
value. The core data server is useful on its own the moment it exists. The risk to avoid is
building Phase 1 in a way that forces a rewrite to reach later phases.

## Decision

Deliver in **independently-useful phases**, and structure the server with a **transport
abstraction (stdio → HTTP) from the start** so later phases don't require a rewrite.

- **Phase 0 — Harness (now).** Skeleton `rmcp` server (one trivial tool) all the way green: CI
  passing, the `just check` verifier stack wired, fixtures + one passing MCP conformance test,
  a Fly.io preview deploying. No real weather logic.
  ([0005](0005-hands-off-agent-development.md))
- **Phase 1 — Data-only Rust MCP (stdio).** The three real tools
  ([0004](0004-minimal-tool-surface.md)) against the Forecast + ERA5 Archive APIs. Claude draws
  charts on demand from the JSON; works in Claude Desktop / CCD today.
- **Phase 2 — MCP App UI components.** Interactive trend chart / anomaly view via the
  `create-mcp-app` skill + `@modelcontextprotocol/ext-apps`. The UI is a Node/Vite-built HTML
  bundle served by the Rust server — a separate web-frontend workstream, distinct from the Rust
  server itself.
- **Phase 3 — Fly.io + OAuth → mobile (the real game-changer).** A phone can't reach a local
  stdio binary; mobile means a remote, OAuth-authenticated server (discovery +
  protected-resource metadata + JWT validation). Real infra lift, not a flag flip.

## Why phased + transport abstraction

- **Each phase is independently useful.** Phase 1 alone answers every forecast and trend
  question from the desktop — the original goal — without waiting on viz or mobile.
- **No rewrite tax.** Abstracting the transport (stdio now, HTTP later) means Phase 3's remote
  server reuses the Phase 1 tool implementations rather than re-deriving them.
- **Decouples the hard/uncertain parts.** Viz (web frontend) and remote+OAuth (infra) are
  isolated to later phases instead of blocking the core.

## Gating criteria (verify before investing)

- **Before Phase 2:** deploy a *trivial* MCP App and confirm whether **CCD renders MCP App UI
  inline** before building real viz components. CCD is part of the Claude Desktop family, so it
  likely renders where the work happens — but verify empirically.
- **Phase 3 go/no-go:** does **Claude mobile render MCP App UI resources at all**, or only call
  tools? Mobile rendering is the Phase-3 game-changer criterion.

## Consequences

- Phase 1 ships and is dogfooded long before Phase 2/3 exist.
- The transport seam must be designed into the Phase 1 architecture
  ([ARCHITECTURE.md](../guides/ARCHITECTURE.md)), even though only stdio is exercised at first.
- Deploy steps in Phase 3 (and Fly.io first-run) are human-in-loop
  ([0003](0003-standalone-public-repo.md)).

## Alternatives considered

- **Build viz / remote up front** — rejected: stalls the core value behind the two hardest,
  most uncertain workstreams.
- **stdio-only, no transport abstraction** — rejected: cheap now, but forces a Phase 3 rewrite
  of the serving layer.

## See also

- [0004 — Minimal tool surface](0004-minimal-tool-surface.md)
- [0005 — Hands-off agent development](0005-hands-off-agent-development.md)
- [Roadmap](../product/roadmap.md)
