# Roadmap

The phased plan ([0006](../decisions/0006-phased-delivery.md)) and the questions still to
settle. Each phase is independently useful; the transport abstraction keeps later phases from
forcing a rewrite.

## Phases

### Phase 0 — Harness (in progress)

Skeleton `rmcp` server (one trivial tool) all the way green: CI passing, `just check` verifier
stack wired, fixtures + one passing MCP conformance test, a Fly.io preview deploying. **No real
weather logic.** Target: green CI on a skeleton. See [now.md](now.md) for the concrete step
list and [DEVELOPMENT.md](../guides/DEVELOPMENT.md) for the verifier bar.

### Phase 1 — Data-only Rust MCP (stdio)

The three real tools ([0004](../decisions/0004-minimal-tool-surface.md)):
`get_forecast`, `get_historical`, `compare_period` — against the Open-Meteo Forecast + ERA5
Archive APIs, cribbing parameter shapes from `cmer81/open-meteo-mcp`. Claude draws charts on
demand from the JSON. Works in Claude Desktop / CCD today.

### Phase 2 — MCP App UI components

Interactive trend chart / anomaly view via the `create-mcp-app` skill +
`@modelcontextprotocol/ext-apps` (a Node/Vite HTML bundle served by the Rust server).
**Gate:** first deploy a trivial MCP App and confirm CCD renders it inline before investing.

### Phase 3 — Fly.io + OAuth → mobile

Remote, OAuth-authenticated server (discovery + protected-resource metadata + JWT validation)
so Claude mobile can reach it — the real game-changer. Real infra lift. **Gate:** confirm
Claude mobile actually renders MCP App UI resources (not just calls tools).

## Open questions

These are unsettled and should be resolved before or while the relevant phase is built:

- **Variables (`vars`)** — beyond precipitation + temperature, which? (snow, wind, humidity?)
- **`compare_period` baseline** — trailing N years, or a 30-year climate normal? And which
  summary stats — mean, anomaly vs. baseline, percentile rank?
- **Default-location handling** — pass `lat`/`lon` on every call, or add a small place→coords
  resolver / saved home location?
- **Archive API rate limits + caching** — how to stay within limits and avoid re-fetching the
  same historical windows.
- **CCD renders MCP App UI?** (Phase 2 gate) — verify empirically with a trivial MCP App.
- **Claude mobile renders MCP App UI resources?** (Phase 3 go/no-go) — or does it only call
  tools?

## Done / decided

- Data source → Open-Meteo ([0001](../decisions/0001-data-source-open-meteo.md))
- Build in Rust on `rmcp` 1.7.0 ([0002](../decisions/0002-build-in-rust-with-rmcp.md))
- Standalone public repo, Apache-2.0, Fly.io ([0003](../decisions/0003-standalone-public-repo.md))
- Three-tool surface ([0004](../decisions/0004-minimal-tool-surface.md))
- Hands-off via verifiers + harness-first ([0005](../decisions/0005-hands-off-agent-development.md))
- Phased delivery + transport abstraction ([0006](../decisions/0006-phased-delivery.md))
