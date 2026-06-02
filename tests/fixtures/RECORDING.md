# Recording the test fixtures

The deterministic test suite ([test-plan §2](../../docs/design/test-plan.md#2-fixtures-testsfixtures))
runs against **recorded real Open-Meteo responses** committed here, so parsing/aggregation is
offline and reproducible. This file is the capture recipe.

> **Who runs this:** a human, once. Agents can't reach the network with `curl`/`wget` (denied),
> and `WebFetch` mangles large JSON, so the four real captures below are recorded by hand and
> committed. The `error_*.json` fixtures are **crafted** (see bottom) and committed by the agent;
> the edge cases (leap-year window, ERA5-lag boundary) are **sliced in-test** from the wide
> archive capture — no extra captures needed.

All commands assume you're in the repo root. City is **Boston** (`42.3584, -71.0598`), the running
example throughout the specs. Units are left at the Open-Meteo default (**metric**). Dates are
fixed (not "today") so the fixtures stay deterministic; `2026-05-25` is a safe archive end given
the ~5-day ERA5 lag.

```sh
# 1. Geocoding — several "Boston" hits (resolution + ambiguity note, test-plan §3.3)
curl -s 'https://geocoding-api.open-meteo.com/v1/search?name=Boston&count=10&language=en&format=json' \
  | jq . > tests/fixtures/geocode_boston.json

# 2. Geocoding — zero results (-> location_not_found)
curl -s 'https://geocoding-api.open-meteo.com/v1/search?name=Zzqxnowhereplace&count=10&language=en&format=json' \
  | jq . > tests/fixtures/geocode_empty.json

# 3. Forecast — 7-day, the exact current+daily variable set from tool-specs §2
curl -s 'https://api.open-meteo.com/v1/forecast?latitude=42.3584&longitude=-71.0598&timezone=auto&forecast_days=7&current=temperature_2m,relative_humidity_2m,apparent_temperature,precipitation,weather_code,wind_speed_10m,wind_direction_10m,wind_gusts_10m,cloud_cover,is_day&daily=weather_code,temperature_2m_max,temperature_2m_min,precipitation_sum,precipitation_probability_max,wind_speed_10m_max' \
  | jq . > tests/fixtures/forecast_boston.json

# 4. Archive (ERA5) — ONE wide window covering the 1991-2020 default baseline AND a 2026 period,
#    all four curated variables' daily columns. The FixtureClient slices this per request, so this
#    single capture feeds get_historical, the compare_period baseline, and the period. (~1.5-2 MB.)
curl -s 'https://archive-api.open-meteo.com/v1/archive?latitude=42.3584&longitude=-71.0598&start_date=1991-01-01&end_date=2026-05-25&daily=temperature_2m_max,temperature_2m_min,temperature_2m_mean,precipitation_sum,snowfall_sum,wind_speed_10m_max&timezone=auto' \
  | jq . > tests/fixtures/archive_boston_1991-2026.json
```

## After capturing

- Sanity-check each file is valid JSON with the expected top-level keys:
  - geocode: `results` array (or absent for the empty one), each hit with `latitude`/`longitude`/`name`/`country_code`/`admin1`/`population`/`timezone`.
  - forecast: `current`, `current_units`, `daily`, `daily_units`, `timezone`, `latitude`, `longitude`, `elevation`.
  - archive: `daily` with `time` + the six requested columns, all equal-length; `daily_units`.
- **Confirm `temperature_2m_mean` is present in the archive `daily`** — the one
  [verify-at-build flag](../../docs/design/tool-specs.md#14-the-curated-variable-set). If it's
  missing, capture without it and tell me (the impl derives the mean as `(max+min)/2` + a note).
- Leave the files `jq`-pretty-printed (stable diffs). Then ping me and I'll compute the
  `compare.rs` oracle numbers off these files and finalize the red tests.

## Crafted fixtures (agent-owned, committed separately)

- `error_upstream.json` — `{"error":true,"reason":"…"}` (Open-Meteo error body → `upstream_error`).
- `error_rate_limited.json` — representative 429 body; the HTTP-429 → `upstream_rate_limited`
  mapping is status-driven and unit-tested with the status code passed in.
