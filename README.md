# weather-mcp

A standalone, open-source **Rust MCP server** wrapping the [Open-Meteo](https://open-meteo.com)
API — so you can ask Claude not just *"what's the forecast"* but *"how does this spring's
rainfall compare to the 30-year normal — or the trend since 1950?"* The historical-trend
question is the point: it's backed by ERA5 reanalysis going back to 1940, which no consumer
weather app exposes.

> **Status: early — design frozen, build next.** The repo holds the design (decision records,
> roadmap, guides, and now the [frozen tool + app specs](docs/design/)) plus the agent config.
> **No Rust weather logic yet, by design** — the harness and the specs come before the features;
> Phase 2 builds the three tools against those specs. See [docs/product/now.md](docs/product/now.md).

## Read this first

- [AGENTS.md](AGENTS.md) — how agents (Claude Code + Codex) work in this repo
- [docs/product/now.md](docs/product/now.md) — current focus + the next concrete step
- [docs/product/roadmap.md](docs/product/roadmap.md) — phased plan + open questions
- [docs/design/](docs/design/) — the frozen tool + app contracts Phase 2 builds against
- [docs/guides/ARCHITECTURE.md](docs/guides/ARCHITECTURE.md) — planned crate layout + transport seam
- [docs/guides/DEVELOPMENT.md](docs/guides/DEVELOPMENT.md) — the verifier bar (`just check`, CI, tests)
- [docs/decisions/](docs/decisions/) — why it's shaped this way (ADRs 0001–0007)

## The planned shape

Three tools, deliberately small ([ADR 0004](docs/decisions/0004-minimal-tool-surface.md)) — full
contract in [docs/design/tool-specs.md](docs/design/tool-specs.md):

- `get_forecast` — current conditions + N-day forecast
- `get_historical` — daily history from the ERA5 archive
- `compare_period` — a period vs. a climate baseline (default the 1991–2020 normal; widen to any
  range back to 1940 for the long-term trend)

Each takes a location as a **place name *or* `lat`/`lon`**. Built on the official
[`rmcp`](https://crates.io/crates/rmcp) SDK + tokio, served over stdio first and growing to a
remote OAuth server (for Claude mobile) later — without a rewrite.

## Why this exists

It's also a deliberate experiment in **hands-off development with agents**: favor verifiers
(the Rust compiler, `clippy -D warnings`, an MCP conformance test, CI) over human approval
gates, and build the harness before the features. See
[ADR 0005](docs/decisions/0005-hands-off-agent-development.md).

## License

[Apache-2.0](LICENSE).
