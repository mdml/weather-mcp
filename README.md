# weather-mcp

A standalone, open-source **Rust MCP server** wrapping the [Open-Meteo](https://open-meteo.com)
API — so you can ask Claude not just *"what's the forecast"* but *"how does this year's
rainfall compare to the last ten years?"* The historical-trend question is the point: it's
backed by ERA5 reanalysis going back to 1940, which no consumer weather app exposes.

> **Status: early.** This repo currently holds the design (decision records, roadmap, guides)
> and the agent config — **no Rust code yet, by design**: the harness comes before the weather
> logic. See [docs/product/now.md](docs/product/now.md).

## Read this first

- [AGENTS.md](AGENTS.md) — how agents (Claude Code + Codex) work in this repo
- [docs/product/now.md](docs/product/now.md) — current focus + the next concrete step
- [docs/product/roadmap.md](docs/product/roadmap.md) — phased plan + open questions
- [docs/guides/ARCHITECTURE.md](docs/guides/ARCHITECTURE.md) — planned crate layout + transport seam
- [docs/guides/DEVELOPMENT.md](docs/guides/DEVELOPMENT.md) — the verifier bar (`just check`, CI, tests)
- [docs/decisions/](docs/decisions/) — why it's shaped this way (ADRs 0001–0006)

## The planned shape

Three tools, deliberately small ([ADR 0004](docs/decisions/0004-minimal-tool-surface.md)):

- `get_forecast(lat, lon)` — current conditions + N-day forecast
- `get_historical(lat, lon, start, end, vars)` — daily history from the ERA5 archive
- `compare_period(lat, lon, this_year_range, baseline_years)` — this year vs. the past decade

Built on the official [`rmcp`](https://crates.io/crates/rmcp) SDK + tokio, served over stdio
first and growing to a remote OAuth server (for Claude mobile) later — without a rewrite.

## Why this exists

It's also a deliberate experiment in **hands-off development with agents**: favor verifiers
(the Rust compiler, `clippy -D warnings`, an MCP conformance test, CI) over human approval
gates, and build the harness before the features. See
[ADR 0005](docs/decisions/0005-hands-off-agent-development.md).

## License

[Apache-2.0](LICENSE).
