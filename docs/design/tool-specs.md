# Tool specs — the Phase 2 contract

> **Status: frozen for the Phase 2 build, expected to evolve.** This is the contract Phase 2
> builds against and the `insta` snapshots pin — frozen enough that an agent can grind against a
> stable bar, *not* frozen as in "we got it right the first time." It's a humble v1; we expect to
> revise it as we dogfood. Changing a request param or output field is a deliberate spec change
> (amend this doc + the snapshots together), not silent drift from code.
> See [0004](../decisions/0004-minimal-tool-surface.md) · [0006](../decisions/0006-phased-delivery.md) ·
> [ARCHITECTURE](../guides/ARCHITECTURE.md) · [app-spec](app-spec.md).

### Design stance — flexible defaults over premature precision

We don't yet know exactly how we'll want to slice this data, so the design favors **flexibility
with sensible defaults** over locking in choices we can't justify yet:

- **Defaults are starting points, not constraints.** Every "default" below (1991–2020 baseline,
  the curated variable set, sum-vs-mean aggregation) is the obvious first guess — overridable by
  a parameter, and cheap to revisit.
- **Wide knobs where we're unsure.** The baseline is *any* year range back to 1940, not a fixed
  enum of presets — so "show me the trend back to 1950" is a first-class request today
  ([§4.1](#41-baseline--a-reference-window-default-the-climate-normal)), not a future feature.
- **Adding is cheap, removing is expensive** — so we start lean (three tools, four variables)
  and grow only when a real need appears, rather than guessing broad now.

Three tools, no more ([0004](../decisions/0004-minimal-tool-surface.md)):

| Tool | Answers | Open-Meteo endpoint(s) |
|---|---|---|
| `get_forecast` | "what's the weather going to be" | Forecast |
| `get_historical` | "give me the daily record for this window" | Archive (ERA5) |
| `compare_period` | "how does this period compare to the climate normal" | Archive (ERA5) + Geocoding |

`compare_period` is the differentiator — the deterministic, server-side aggregation that no
other Open-Meteo MCP server exposes. The other two are deliberately thin wrappers.

---

## 1. Shared conventions

### 1.1 Location — name *or* coordinates

Every tool takes location the same way. Provide **exactly one** of:

- `location` *(string)* — a place name (e.g. `"Boston"`, `"Boston, MA"`, `"Reykjavík"`).
  Resolved server-side via the Geocoding API (`geocoding-api.open-meteo.com/v1/search`,
  `count=…`, top result by population wins).
- `latitude` + `longitude` *(floats, WGS84)* — the unambiguous escape hatch.

Rules:

- Supplying neither, or `location` together with `latitude`/`longitude`, is an
  `invalid_request` error (§1.5).
- **Resolution is always echoed** in the output `location` object (§1.6) so a wrong
  "Springfield" is visible and correctable. When more than one strong match exists, the chosen
  one is returned and the alternatives are listed in `notes` — **ambiguity is a note, not an
  error** (non-blocking).
- A `location` string that geocodes to zero results is a `location_not_found` error.
- Reverse geocoding is **not** done in Phase 2: when coordinates are passed directly, the
  echoed `location.name`/`admin1`/`country` are `null` (the timezone + elevation still come back
  from the weather response). Reverse geocoding is a possible future add.

### 1.2 Units

A single `units` param keeps the model from juggling three:

| `units` | temperature | precipitation / snowfall | wind |
|---|---|---|---|
| `"metric"` *(default)* | °C | mm | km/h |
| `"imperial"` | °F | inch | mph |

Maps to Open-Meteo's `temperature_unit` / `precipitation_unit` / `wind_speed_unit`. The
resolved unit labels are echoed in every output `units` object so the model never guesses.

### 1.3 Timezone & daily alignment

All requests pass **`timezone=auto`** so "daily" buckets are aligned to the location's *local*
calendar day (not UTC) — essential for the per-day aggregation in `compare_period` to mean what
a person means by "a day". The resolved IANA timezone is echoed in `location.timezone`.

### 1.4 The curated variable set

The historical/comparison path uses a **fixed enum** of clean names mapped to Open-Meteo
columns — small, sharp, and snapshot-testable (the anti-sprawl decision of
[0004](../decisions/0004-minimal-tool-surface.md)). `get_forecast` is *not* limited to this set;
near-term conditions are cheap, so it returns a richer fixed payload (§2).

| Enum (`variables[]`) | compare scalar | aggregation | `get_historical` daily columns (Open-Meteo) |
|---|---|---|---|
| `temperature` *(default)* | mean of daily mean temp | **mean** | `temperature_2m_max`, `temperature_2m_min`, `temperature_2m_mean` |
| `precipitation` *(default)* | period total | **sum** | `precipitation_sum` |
| `snowfall` | period total | **sum** | `snowfall_sum` |
| `wind` | mean of daily max wind | **mean** | `wind_speed_10m_max` |

Humidity and other forecast-only fields are intentionally absent here: daily humidity isn't a
first-class ERA5 aggregate and the trend story doesn't need it. Adding a variable = one row here
+ one snapshot.

> **Verify at build:** `temperature_2m_mean` is confirmed available on the Forecast API; on the
> **Archive** API it's expected but the docs page only listed it partially. The Phase 2 agent
> confirms it against the live archive (the live/snapshot tests catch a miss); if it's absent,
> derive the mean as `(temperature_2m_max + temperature_2m_min) / 2` and note the derivation.

### 1.5 Error model

User-actionable failures return a **`CallToolResult` with `is_error: true`** and a structured
JSON body (so the model can read and recover), not a protocol-level error:

```json
{ "error": { "code": "location_not_found", "message": "No place matches \"Atlantis\".", "details": {} } }
```

| `code` | When |
|---|---|
| `invalid_request` | neither/both of location vs lat/lon; malformed params |
| `location_not_found` | geocoding returned zero results |
| `invalid_date_range` | `start > end`; `start` before `1940-01-01`; end in the future (historical/compare) |
| `upstream_rate_limited` | Open-Meteo HTTP 429 |
| `upstream_error` | Open-Meteo `{"error":true,"reason":…}` or 5xx (the `reason` is passed through in `message`) |
| `upstream_unavailable` | network failure / timeout reaching Open-Meteo |

Non-fatal conditions (location ambiguity, ERA5-lag clamp §1.7, dropped Feb 29) are **`notes`**,
not errors — the call still succeeds.

### 1.6 Shared output envelope

Every successful result carries:

```jsonc
{
  "location": {
    "name": "Boston", "admin1": "Massachusetts",
    "country": "United States", "country_code": "US",
    "latitude": 42.3584, "longitude": -71.0598,
    "elevation": 38.0, "timezone": "America/New_York",
    "source": "geocoded"            // "geocoded" | "coordinates"
  },
  "units": { "temperature": "°C", "precipitation": "mm", "wind_speed": "km/h" },
  "notes": []                        // human-readable strings: clamps, ambiguity, dropped days
  // ...tool-specific payload...
}
```

### 1.7 The ERA5 lag (archive tools)

ERA5 trails real-time by **~5 days**. For `get_historical`/`compare_period`, if a requested end
date is later than the last available archive date, **clamp** it to that date and add a `notes`
entry (`"end clamped from 2026-05-30 to 2026-05-25 (ERA5 5-day lag)"`) — never silently return
short data, never error. Filling the gap from the Forecast API's `past_days` is a deferred
option.

---

## 2. `get_forecast`

Current conditions + an N-day daily forecast.

**Request**

| Param | Type | Default | Notes |
|---|---|---|---|
| `location` *or* `latitude`+`longitude` | string / float | — | §1.1 |
| `forecast_days` | int 1–16 | `7` | Open-Meteo max is 16 |
| `units` | enum | `"metric"` | §1.2 |

**Output** (+ shared envelope §1.6)

```jsonc
{
  "current": {
    "time": "2026-05-30T14:00",
    "temperature": 19.4, "apparent_temperature": 18.9,
    "relative_humidity": 57, "precipitation": 0.0,
    "weather_code": 3, "weather": "Overcast",        // WMO code + decoded label
    "wind_speed": 14.2, "wind_direction": 250, "wind_gusts": 28.1,
    "cloud_cover": 88, "is_day": true
  },
  "daily": {                                          // columnar — chart-friendly, length == forecast_days
    "time":               ["2026-05-30", "2026-05-31", …],
    "weather_code":       [3, 61, …],
    "temperature_max":    [21.0, 18.5, …],
    "temperature_min":    [11.2, 12.0, …],
    "precipitation_sum":  [0.0, 6.4, …],
    "precipitation_probability_max": [10, 80, …],
    "wind_speed_max":     [18.0, 24.0, …]
  }
}
```

- `weather_code` is the raw WMO code; `weather` is the decoded label from a small static WMO
  table owned by the server (so the model/app needn't carry one). The decode table is part of
  this contract — see Appendix B.
- Open-Meteo wire variables requested: `current=temperature_2m,relative_humidity_2m,`
  `apparent_temperature,precipitation,weather_code,wind_speed_10m,wind_direction_10m,`
  `wind_gusts_10m,cloud_cover,is_day`; `daily=weather_code,temperature_2m_max,`
  `temperature_2m_min,precipitation_sum,precipitation_probability_max,wind_speed_10m_max`.

## 3. `get_historical`

The daily record for an explicit window — a thin, faithful pass-through of the ERA5 daily
arrays for the curated variables.

**Request**

| Param | Type | Default | Notes |
|---|---|---|---|
| `location` *or* `latitude`+`longitude` | string / float | — | §1.1 |
| `start_date` | `YYYY-MM-DD` | — | ≥ `1940-01-01` |
| `end_date` | `YYYY-MM-DD` | — | clamped per §1.7 |
| `variables` | enum[] | `["temperature","precipitation"]` | §1.4 |
| `units` | enum | `"metric"` | §1.2 |

**Output** (+ envelope §1.6) — Open-Meteo's columnar `{time:[], <col>:[]}` shape preserved (all
arrays equal length, index-aligned), expanded to the curated columns of each requested variable
(§1.4):

```jsonc
{
  "range": { "start": "2020-01-01", "end": "2020-12-31" },
  "daily": {
    "time":               ["2020-01-01", …],
    "temperature_2m_max": [4.1, …], "temperature_2m_min": [-1.2, …], "temperature_2m_mean": [1.5, …],
    "precipitation_sum":  [2.3, …]
  }
}
```

## 4. `compare_period` — the differentiator

Aggregate a period of interest and compare it against a **fixed climate baseline**, using
**calendar-window matching**: the same month-day window is extracted from every baseline year,
giving a distribution to score the period against. Deterministic and fixture-tested
([ARCHITECTURE](../guides/ARCHITECTURE.md) — `compare.rs` is pure).

### 4.1 Baseline = a reference window (default: the climate normal)

The baseline is **any year range** — `baseline_start_year` / `baseline_end_year`, any pair from
1940 onward — so we're not betting the design on one definition of "normal". A few reference
points worth naming:

| `baseline_start_year` / `baseline_end_year` | Meaning |
|---|---|
| `1991` / `2020` *(default)* | Current **WMO 30-year climatological normal** — a sensible, conventional default |
| `1951` / `1980` | NASA-GISS-style **pre-warming reference** (a common climate-change framing) |
| `1950` / `2025` | The **long view** — every year since 1950 as a multi-decade trend (see below) |
| any custom pair ≥ 1940 | Whatever window the question calls for |

**Why a fixed window is the *default* (not a constraint):** a trailing-N-years baseline slides
forward with the warming climate, so it quietly masks the very trend we're trying to see — a
fixed reference period doesn't. That's the reason 1991–2020 is the default rather than "last 10
years"; it is **not** a reason to forbid anything. Want a trailing window, a recent decade, or
1950→present? Set the years. (Pre-industrial 1850–1900 is the one thing genuinely out of reach —
ERA5 starts in 1940.)

**The long view falls out for free.** Because the output carries the **per-year**
`baseline.values` array (§4.4), widening the baseline to e.g. 1950–2025 turns `compare_period`
into a multi-decade **trend explorer**: 76 yearly points plotted, with the current period
highlighted against them — no separate tool needed. This is exactly the "comparisons going back
to 1950" use case, and it's why the per-year array (not just summary stats) is in the contract.

### 4.2 Calendar-window matching

- `period` defines a month-day window from its `start`/`end` (e.g. `01-01 … 05-30`).
- For each year `Y` in `[baseline_start_year, baseline_end_year]`, the **same MM-DD window
  within year Y** is aggregated → one baseline value per year per variable.
- **Cross-year windows:** if `period.end`'s MM-DD precedes `period.start`'s (e.g. a Dec 1 –
  Feb 28 winter), the window wraps into the next calendar year; for baseline year `Y` it spans
  `Dec Y → Feb Y+1`.
- **Feb 29** is included in the years that have it; aggregation is over the actual days present.
  For a multi-decade normal the leap-day effect on a sum is negligible and on a mean nil; a
  `notes` entry records it when relevant.
- Per-variable aggregation (sum vs mean) is fixed by §1.4.

### 4.3 Request

| Param | Type | Default | Notes |
|---|---|---|---|
| `location` *or* `latitude`+`longitude` | string / float | — | §1.1 |
| `period` | `{ start, end }` `YYYY-MM-DD` | — | the window of interest; `end` clamped §1.7 |
| `variables` | enum[] | `["temperature","precipitation"]` | §1.4 |
| `baseline_start_year` | int ≥ 1940 | `1991` | §4.1 |
| `baseline_end_year` | int ≥ 1940 | `2020` | §4.1 |
| `include_series` | bool | `false` | adds day-by-day series for the time-series view (§4.5) |
| `units` | enum | `"metric"` | §1.2 |

### 4.4 Output (+ envelope §1.6)

```jsonc
{
  "period":   { "start": "2026-01-01", "end": "2026-05-25", "window": "01-01..05-25" },
  "baseline": { "reference": "1991-2020", "start_year": 1991, "end_year": 2020, "n_years": 30 },
  "comparisons": [
    {
      "variable": "precipitation", "unit": "mm", "aggregation": "sum",
      "period_value": 412.3,
      "baseline": {
        "mean": 357.1, "stddev": 64.2, "min": 240.0, "max": 502.0,
        "values": [ { "year": 1991, "value": 333.0 }, { "year": 1992, "value": 410.5 }, … ]
      },
      "anomaly": {
        "absolute": 55.2,            // period_value − baseline.mean
        "percent": 15.5,             // relative to baseline.mean
        "standardized": 0.86,        // z-score: (period_value − mean) / stddev
        "percentile_rank": 81.0,     // percentile of period_value within the baseline distribution
        "rank": "6th-wettest of 31"  // period ranked among baseline years (+ the period itself)
      }
    }
    // …one entry per requested variable
  ],
  "series": null                     // populated only when include_series=true (§4.5)
}
```

`baseline.values` (the per-year array) is the key shape decision: it lets the Phase 3 app draw
the **distribution**, not just the headline number — see [app-spec](app-spec.md).

### 4.5 Optional day-by-day series (`include_series=true`)

Adds the data the within-period time-series view needs in **one call** (instead of a second
`get_historical`): the period's daily curve plus the baseline's per-day-of-window mean (a
climatology band).

```jsonc
"series": {
  "window_days": ["01-01", "01-02", …],            // the MM-DD axis
  "period":   { "precipitation_sum": [1.2, 0.0, …] },        // this period, per day
  "baseline_daily": { "precipitation_sum": { "mean": [0.9, 1.1, …], "stddev": [0.4, 0.6, …] } }
}
```

### 4.6 API-call budget & caching

- One archive request fetches the **entire baseline window in a single call** (e.g. all of
  1991–2020) and is sliced client-side; the period is a second request; geocoding is one more
  for a `location` string. So a `compare_period` call is **~2–3 upstream requests** — trivially
  within the free limits (600/min). No per-year fan-out.
- **Caching is deferred** (Phase 2 ships without it) but the seam lives in the `openmeteo/`
  client: archive windows older than the 5-day lag are **immutable**, and the 1991–2020 normal
  for a given location is the prime cache candidate (identical across repeated calls). Document,
  don't build yet.

---

## Appendix A — Open-Meteo endpoints

| Purpose | URL |
|---|---|
| Forecast | `https://api.open-meteo.com/v1/forecast` |
| Archive (ERA5) | `https://archive-api.open-meteo.com/v1/archive` |
| Geocoding | `https://geocoding-api.open-meteo.com/v1/search` |

Keyless ([0001](../decisions/0001-data-source-open-meteo.md)). Free limits: 600/min · 5 000/hr ·
10 000/day · 300 000/mo. Errors arrive as `{"error":true,"reason":"…"}`; rate-limiting as HTTP
429 (both mapped in §1.5).

## Appendix B — WMO weather codes (decode table)

The server owns the WMO 4677 code→label map used for `weather` (§2). Abbreviated; the full
table is pinned in code + snapshot:

| Code | Label | | Code | Label |
|---|---|---|---|---|
| 0 | Clear sky | | 61/63/65 | Rain: slight/moderate/heavy |
| 1/2/3 | Mainly clear / Partly cloudy / Overcast | | 71/73/75 | Snow: slight/moderate/heavy |
| 45/48 | Fog / Depositing rime fog | | 80/81/82 | Rain showers: slight/moderate/violent |
| 51/53/55 | Drizzle: light/moderate/dense | | 95/96/99 | Thunderstorm / with hail |
