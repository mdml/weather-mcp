# 0001 — Data source: Open-Meteo (Forecast + ERA5 Archive)

**Date:** 2026-05-30
**Status:** active

## Context

The whole point of this project is the historical-trend question: *"how does this year's
rainfall compare to the last ten years?"* — not just *"what's tomorrow."* The MCP wrapper is
incidental; **the data source behind it is the decision that matters.**

Most weather APIs are forecast-only. They can tell you the next 7–16 days but expose no deep,
queryable history, which makes the 1-year-vs-10-year comparison impossible. The curated MCP
connector registry has no weather connectors at all (it's a SaaS/productivity directory), so
any weather MCP is a hand-installed community server regardless — there's no one-click path to
preserve.

## Decision

Back the server with **[Open-Meteo](https://open-meteo.com)**, using two of its APIs:

- **Forecast API** — current conditions + N-day forecast.
- **Historical Weather (Archive) API** — daily history from **ERA5 reanalysis, back to 1940**.

Open-Meteo is free, requires **no API key**, and is global. A single Open-Meteo-backed server
answers both the forecast question and the multi-decade trend question.

## Why Open-Meteo

- **It does both forecast and deep history.** It is the one surveyed source that exposes a
  multi-decade queryable archive (ERA5, 1940→present) *and* a forecast API.
- **Free + keyless + global.** No key management, no per-call cost, works anywhere. This also
  simplifies the security posture — there's no secret to store (see
  [0003](0003-standalone-public-repo.md)).
- **Gold-standard underlying models.** It serves ECMWF IFS and DWD ICON among others, and its
  `best_match` auto-selects the best regional model.

## Why not the forecast-only sources

- **US NWS, OpenWeatherMap free tier, AccuWeather** — great for "what's the forecast," but
  zero deep history. Most community Open-Meteo MCPs and other weather MCPs wrap these, which is
  exactly why they can't answer the trend question.

## Consequences

- **Honest limitations to document.** Open-Meteo serves near-raw model output; it lacks the
  proprietary post-processing that drives most real accuracy gains (~20% RMSE from
  post-processing vs. ~3–4% from the underlying model choice), hyperlocal minute-by-minute
  nowcasting, and integrated severe-weather alerts. Division of labor: keep a phone weather app
  for the glanceable nowcast + alerts; this server is for forecast + every historical/trend
  question.
- **Two API surfaces** (forecast + archive) with different endpoints and base URLs, so the
  client layer must handle both. Archive **rate-limits and caching** are an open question (see
  [roadmap](../product/roadmap.md)).
- Variables beyond precipitation + temperature (snow, wind, humidity) are an open question.

## Alternatives considered

- **NASA Worldview** — the most literal "Google Earth + layers + time slider," but satellite
  imagery, not historical stats.
- **NOAA Climate at a Glance** — bullseye for "this year vs. history" at city/county/state
  level, but a web tool, not an API to wrap.
- **PRISM Climate Explorer** — high-res gridded US precip/temp time series; US-only, web tool.
- **ClimateEngine.org / Google Earth Engine** — powerful compare-to-baseline analysis built on
  Earth Engine, but no-code map tools / scriptable petabyte archive respectively — far heavier
  than a small keyless API.

These web tools were surveyed for the original "explore local climate patterns" framing before
it narrowed to "just give Claude a weather MCP and ask it directly."

## See also

- [0002 — Build in Rust with rmcp](0002-build-in-rust-with-rmcp.md)
- [0004 — Minimal tool surface](0004-minimal-tool-surface.md)
