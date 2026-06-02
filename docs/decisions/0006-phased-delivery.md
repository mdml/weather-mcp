# 0006 — Phased delivery + transport abstraction

**Date:** 2026-05-30
**Status:** active

> **Amended 2026-05-30:** inserted **Phase 1 — Design** (spec the interfaces before building);
> the former Phases 1–3 (data tools, MCP apps, remote) shifted to **2–4**. Phase 0 (harness
> skeleton + green CI) has landed; the Docker/Fly preview + git hooks were deferred to
> follow-ups rather than bundled into Phase 0.
>
> **Amended 2026-06-01:** split the build into **Phase 2 — Test harness + executable bar** (set
> the verifier bar first, human-reviewed) and **Phase 3 — Tool implementation** (the hands-off
> red→green grind); MCP apps shifted to **4**, remote to **5**. Rationale: the
> [0005](0005-hands-off-agent-development.md) bet wants the human reviewing *the bar*, then
> trusting a green checkmark — so we write the tests (against the frozen Phase 1 specs) before
> the implementation, making the build a clean grind with no taste calls left. Writing the bar
> first also forces the specs to become concrete (exact JSON, rounding, error bodies), surfacing
> ambiguity while it's cheap. The split is not code-free: compiling tests need the type surface +
> `WeatherData` trait seam as stubs, and `insta` snapshots are *accepted* in Phase 3, not
> hand-authored in Phase 2 — so the Phase 2 bar is the hand-asserted tests (esp. the pure
> `compare.rs` aggregation, checked against an independent oracle) + the un-mockable conformance
> test. See [test-plan](../design/test-plan.md).

## Context

Visualization and remote/mobile access are tempting to chase early, but each is a different
kind of lift (a web-frontend workstream; a real infra + auth lift) and would stall the core
value. The core data server is useful on its own the moment it exists. The risk to avoid is
building the core tools (Phase 2) in a way that forces a rewrite to reach later phases.

## Decision

Deliver in **independently-useful phases**, and structure the server with a **transport
abstraction (stdio → HTTP) from the start** so later phases don't require a rewrite.

- **Phase 0 — Harness (done).** Skeleton `rmcp` server (one trivial tool) green: CI passing,
  the `just check` verifier stack wired, fixtures + one passing MCP conformance test. No real
  weather logic. (Docker/Fly preview + git hooks deferred to follow-ups.)
  ([0005](0005-hands-off-agent-development.md))
- **Phase 1 — Design (done).** Spec the three tool interfaces (params, output JSON shapes, units,
  error model) and the future MCP-app UX *before* building — so the build is an agent grinding
  against a frozen spec, and the tool outputs are shaped to feed the apps. Resolves the roadmap's
  open questions. Human-led; no fanout. ([0004](0004-minimal-tool-surface.md);
  [tool-specs](../design/tool-specs.md) + [app-spec](../design/app-spec.md))
- **Phase 2 — Test harness + executable bar (now).** Set the verifier bar against the frozen
  Phase 1 specs *before* implementing: record the Open-Meteo fixtures; define the tool type
  surface + the `WeatherData` client trait seam as stubs; write the hand-asserted tests that
  encode the specs (esp. the pure `compare.rs` aggregation, against an independent oracle); extend
  the MCP conformance test to the three tools; scaffold the `insta` snapshot functions. Result: a
  well-defined *red* `just check` — the bar a human reviews once. ([0005](0005-hands-off-agent-development.md);
  [test-plan](../design/test-plan.md))
- **Phase 3 — Tool implementation (the hands-off grind).** Fill in the pure logic behind the seam
  until `just check` is green and snapshots are accepted — the maximally hands-off fanout (one
  agent per tool/module). The three real tools ([0004](0004-minimal-tool-surface.md)) against the
  Forecast + ERA5 Archive APIs. Claude draws charts on demand from the JSON; works in Claude
  Desktop / CCD today.
- **Phase 4 — MCP App UI components.** The everyday forecast view + the trend/anomaly view
  ([app-spec](../design/app-spec.md)) via the `create-mcp-app` skill +
  `@modelcontextprotocol/ext-apps`. The UI is a Node/Vite-built HTML bundle served by the Rust
  server — a separate web-frontend workstream, distinct from the Rust server itself.
- **Phase 5 — Fly.io + OAuth → mobile (the real game-changer).** A phone can't reach a local
  stdio binary; mobile means a remote, OAuth-authenticated server (discovery +
  protected-resource metadata + JWT validation). Real infra lift, not a flag flip.

## Why phased + transport abstraction

- **Each phase is independently useful.** The data tools (Phases 2–3) answer every forecast and
  trend question from the desktop — the original goal — without waiting on viz or mobile.
- **No rewrite tax.** Abstracting the transport (stdio now, HTTP later) means Phase 5's remote
  server reuses the Phase 3 tool implementations rather than re-deriving them.
- **Decouples the hard/uncertain parts.** Viz (web frontend) and remote+OAuth (infra) are
  isolated to later phases instead of blocking the core.

## Why design before build (Phase 1)

The tool interfaces and app UX are taste calls — naming, output shapes, units, the
`compare_period` baseline + stats — cheaper to settle on paper than to discover (and re-do)
mid-implementation. Freezing the specs first turns the build (Phases 2–3) into a clean
agent-grind against a known bar ([0005](0005-hands-off-agent-development.md)), and lets the tool
outputs be shaped to feed the Phase 4 apps from the start. It is human-led, not a fanout.

## Gating criteria (verify before investing)

- **Before Phase 4:** deploy a *trivial* MCP App and confirm whether **CCD renders MCP App UI
  inline** before building real viz components. CCD is part of the Claude Desktop family, so it
  likely renders where the work happens — but verify empirically.
- **Phase 5 go/no-go:** does **Claude mobile render MCP App UI resources at all**, or only call
  tools? Mobile rendering is the Phase-5 game-changer criterion.

## Consequences

- The data tools (Phases 2–3) ship and are dogfooded long before Phases 4/5 exist.
- The transport seam must be designed into the Phase 2–3 architecture
  ([ARCHITECTURE.md](../guides/ARCHITECTURE.md)), even though only stdio is exercised at first.
- Deploy steps in Phase 5 (and Fly.io first-run) are human-in-loop
  ([0003](0003-standalone-public-repo.md)).

## Alternatives considered

- **Build viz / remote up front** — rejected: stalls the core value behind the two hardest,
  most uncertain workstreams.
- **stdio-only, no transport abstraction** — rejected: cheap now, but forces a Phase 4 rewrite
  of the serving layer.
- **Build the tools before spec-ing them** — rejected: inverts the leverage; interface/UX taste
  calls are cheaper on paper than discovered (and re-done) mid-build.

## See also

- [0004 — Minimal tool surface](0004-minimal-tool-surface.md)
- [0005 — Hands-off agent development](0005-hands-off-agent-development.md)
- [Roadmap](../product/roadmap.md)
