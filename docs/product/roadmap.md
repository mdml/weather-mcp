# Roadmap

The phased plan ([0006](../decisions/0006-phased-delivery.md)) and the questions still to
settle. Each phase is independently useful; the transport abstraction keeps later phases from
forcing a rewrite.

## Phases

### Phase 0 — Harness (done)

Skeleton `rmcp` server (one trivial `server_info` tool) green: CI passing, the `just check`
verifier stack wired, fixtures dir + one passing MCP conformance test (spawn over stdio:
`initialize` → `tools/list` → `tools/call`), plus `cargo-deny` + `cargo-audit`. No real weather
logic. Landed in [#1](https://github.com/mdml/weather-mcp/pull/1).

**Deferred to follow-ups** (cheap, do when needed): the Docker/Fly preview deploy and the
lefthook/commitlint + dotenvx `just` recipes (the old now.md items 6–7).

### Phase 1 — Design (now)

Spec the interfaces *before* building — human-led, no fanout. Produces design files (under
`docs/design/`) that freeze the contracts Phase 2 builds against:

- **Tool interface specs** for `get_forecast`, `get_historical`, `compare_period`
  ([0004](../decisions/0004-minimal-tool-surface.md)): request params, output JSON shapes,
  units, error model — cribbing parameter shapes from `cmer81/open-meteo-mcp`.
- **MCP-app specs** for the future trend-chart / anomaly view (Phase 3), designed now so the
  Phase 2 tool outputs are shaped to feed them.

This phase resolves the [open questions](#open-questions) below. Freezing the specs turns
Phase 2 into a clean agent-grind against a known bar, and is where the parallel build fanout
gets its frozen contracts.

### Phase 2 — Data-only Rust MCP (stdio)

The three real tools built to the Phase 1 specs, against the Open-Meteo Forecast + ERA5 Archive
APIs. Claude draws charts on demand from the JSON. Works in Claude Desktop / CCD today.

### Phase 3 — MCP App UI components

Interactive trend chart / anomaly view via the `create-mcp-app` skill +
`@modelcontextprotocol/ext-apps` (a Node/Vite HTML bundle served by the Rust server).
**Gate:** first deploy a trivial MCP App and confirm CCD renders it inline before investing.

### Phase 4 — Fly.io + OAuth → mobile

Remote, OAuth-authenticated server (discovery + protected-resource metadata + JWT validation)
so Claude mobile can reach it — the real game-changer. Real infra lift. **Gate:** confirm
Claude mobile actually renders MCP App UI resources (not just calls tools).

## Open questions

These are unsettled and are resolved in **Phase 1 — Design** (or while the relevant phase is
built):

- **Variables (`vars`)** — beyond precipitation + temperature, which? (snow, wind, humidity?)
- **`compare_period` baseline** — trailing N years, or a 30-year climate normal? And which
  summary stats — mean, anomaly vs. baseline, percentile rank?
- **Default-location handling** — pass `lat`/`lon` on every call, or add a small place→coords
  resolver / saved home location?
- **Archive API rate limits + caching** — how to stay within limits and avoid re-fetching the
  same historical windows.
- **CCD renders MCP App UI?** (Phase 3 gate) — verify empirically with a trivial MCP App.
- **Claude mobile renders MCP App UI resources?** (Phase 4 go/no-go) — or does it only call
  tools?

## Done / decided

- Data source → Open-Meteo ([0001](../decisions/0001-data-source-open-meteo.md))
- Build in Rust on `rmcp` 1.7.0 ([0002](../decisions/0002-build-in-rust-with-rmcp.md))
- Standalone public repo, Apache-2.0, Fly.io ([0003](../decisions/0003-standalone-public-repo.md))
- Three-tool surface ([0004](../decisions/0004-minimal-tool-surface.md))
- Hands-off via verifiers + harness-first ([0005](../decisions/0005-hands-off-agent-development.md))
- Phased delivery + transport abstraction ([0006](../decisions/0006-phased-delivery.md))
- Secrets via dotenvx — `GH_TOKEN` in `.env.local` ([0007](../decisions/0007-secrets-via-dotenvx.md))
- **Phase 0 harness landed** — skeleton server + verifier stack + green CI ([#1](https://github.com/mdml/weather-mcp/pull/1))
