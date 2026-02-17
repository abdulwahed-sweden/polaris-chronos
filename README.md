---
title: Polaris Chronos
emoji: 🕌
colorFrom: green
colorTo: yellow
sdk: docker
app_port: 7860
---

<h1 align="center">
  Polaris Chronos
</h1>

<p align="center">
  <strong>The Universal Prayer Time Engine</strong>
</p>

<p align="center">
  <a href="#quick-start"><img src="https://img.shields.io/badge/Rust-2021_Edition-DEA584?logo=rust&logoColor=white" alt="Rust"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-blue.svg" alt="MIT License"></a>
  <a href="#testing"><img src="https://img.shields.io/badge/Tests-96_passing-brightgreen" alt="Tests"></a>
  <a href="#"><img src="https://img.shields.io/badge/Version-1.0.0-purple" alt="Version"></a>
  <a href="https://huggingface.co/spaces/abdulwahed-sweden/polaris-chronos"><img src="https://img.shields.io/badge/Live_Demo-HF_Spaces-yellow?logo=huggingface" alt="Live Demo"></a>
</p>

<p align="center">
  High-precision solar position engine written in Rust.<br>
  Computes prayer times for every location on Earth &mdash; including polar regions.<br>
  Ships with a full web dashboard and RESTful API.
</p>

<p align="center">
  <a href="README.md"><strong>English</strong></a>&nbsp;&nbsp;&nbsp;|&nbsp;&nbsp;&nbsp;<a href="README_AR.md"><strong>العربية</strong></a>
</p>

<p align="center">
  <strong>Live Demo:</strong> <a href="https://abdulwahed-sweden-polaris-chronos.hf.space">abdulwahed-sweden-polaris-chronos.hf.space</a>
</p>

<br>

<table>
<tr>
<td>

Most prayer time libraries break above 65&deg;N. The sun doesn't set, so there is no Maghrib. The sun doesn't rise, so there is no Fajr.

**Polaris doesn't break.** It models the sun as a continuous angular wave and produces a complete, transparent schedule &mdash; everywhere, every day, every edge case.

</td>
</tr>
</table>

<br>

## Why Polaris?

<table>
<thead>
<tr>
<th align="left">Problem</th>
<th align="left">Traditional Libraries</th>
<th align="left">Polaris</th>
</tr>
</thead>
<tbody>
<tr>
<td>Polar Night &mdash; no sunrise</td>
<td>Returns error or blank</td>
<td><strong>Complete virtual schedule</strong></td>
</tr>
<tr>
<td>Midnight Sun &mdash; no sunset</td>
<td>Missing Maghrib / Isha</td>
<td><strong>Adaptive projection</strong> from reference latitude</td>
</tr>
<tr>
<td>Transparency</td>
<td>Black-box output</td>
<td>Every time <strong>labeled with its method</strong></td>
</tr>
<tr>
<td>Confidence</td>
<td>No indication</td>
<td><strong>Scored per event</strong> (1.0 &rarr; 0.5)</td>
</tr>
<tr>
<td>Architecture</td>
<td>Static angle formulas</td>
<td><strong>SPA-based simulation</strong> with wave analysis</td>
</tr>
</tbody>
</table>

<br>

## How It Works

The engine computes the sun's altitude at every minute of the day using the **Solar Position Algorithm** (Jean Meeus). From this continuous wave, it derives prayer events &mdash; even when classical threshold crossings don't exist.

Three computation modes, applied automatically:

<table>
<thead>
<tr>
<th align="left">Mode</th>
<th align="center">Confidence</th>
<th align="left">When</th>
<th align="left">What Happens</th>
</tr>
</thead>
<tbody>
<tr>
<td>&#x1F7E2; <strong>Standard</strong></td>
<td align="center"><code>1.0</code></td>
<td>Sun crosses the required altitude</td>
<td>Direct astronomical calculation</td>
</tr>
<tr>
<td>&#x1F7E1; <strong>Virtual</strong></td>
<td align="center"><code>0.7</code></td>
<td>Twilight angle never reached</td>
<td>Derived from wave nadir / peak timing</td>
</tr>
<tr>
<td>&#x1F534; <strong>Projected</strong></td>
<td align="center"><code>0.5</code></td>
<td>Sunrise or sunset doesn't occur</td>
<td>Duration borrowed from adaptive reference latitude (~45&deg;&ndash;55&deg;)</td>
</tr>
</tbody>
</table>

<br>

Every result carries three fields:

```
time  +  method  +  confidence
```

No hidden logic. No silent fallbacks.

<br>

## Real Output &mdash; Troms&oslash;, Norway &mdash; June 21, 2026

> **Midnight Sun.** The sun never sets. Minimum solar altitude: **+3.1&deg;**

```bash
polaris Tromso --date 2026-06-21
```

<table>
<thead>
<tr>
<th align="left">Prayer</th>
<th align="center">Time</th>
<th align="center">Method</th>
<th align="center">Confidence</th>
<th align="left">Note</th>
</tr>
</thead>
<tbody>
<tr>
<td><strong>Fajr</strong></td>
<td align="center"><code>00:46</code> +1d</td>
<td align="center">Virtual</td>
<td align="center">0.70</td>
<td>Derived from wave nadir</td>
</tr>
<tr>
<td><strong>Sunrise</strong></td>
<td align="center"><code>04:07</code></td>
<td align="center">Projected</td>
<td align="center">0.50</td>
<td>Ref. latitude 54.7&deg;</td>
</tr>
<tr>
<td><strong>Dhuhr</strong></td>
<td align="center"><code>12:46</code></td>
<td align="center">Standard</td>
<td align="center">1.00</td>
<td>Solar noon</td>
</tr>
<tr>
<td><strong>Asr</strong></td>
<td align="center"><code>17:57</code></td>
<td align="center">Standard</td>
<td align="center">1.00</td>
<td>Shadow-length ratio</td>
</tr>
<tr>
<td><strong>Maghrib</strong></td>
<td align="center"><code>21:24</code></td>
<td align="center">Projected</td>
<td align="center">0.50</td>
<td>Ref. latitude 54.7&deg;</td>
</tr>
<tr>
<td><strong>Isha</strong></td>
<td align="center"><code>00:46</code> +1d</td>
<td align="center">Virtual</td>
<td align="center">0.70</td>
<td>Derived from wave nadir</td>
</tr>
</tbody>
</table>

<br>

<details>
<summary><strong>What happened here?</strong></summary>
<br>

The sun stayed above the horizon for 24 hours. No physical sunset or sunrise occurred.

- **Dhuhr & Asr** &mdash; calculated normally. The sun still reaches its peak and casts shadows.
- **Sunrise & Maghrib** &mdash; projected from reference latitude 54.7&deg; (no real horizon crossing).
- **Fajr & Isha** &mdash; derived from the solar wave's virtual nadir (twilight angle never reached).

Polaris detected `MidnightSun` state and applied `Projected45` strategy automatically.

</details>

<br>

## Web Dashboard

Polaris ships with a built-in web dashboard featuring:

- **GPS auto-detect** &mdash; finds your nearest city or uses exact coordinates
- **Weekly / Monthly / Daily** calendar views starting from today
- **3-column date layout** &mdash; Day name, Gregorian date, Hijri date
- **Friday highlighting** &mdash; subtle emerald tint for Jumu'ah rows
- **Now dashboard** &mdash; current prayer + countdown to next
- **Horizon dial** &mdash; SVG visualization of the sun's path
- **Interactive day view** &mdash; click any row for full details with confidence bars
- **City search** with autocomplete and multi-city disambiguation
- **API docs** &mdash; built-in developer documentation at `/docs`

### Live Demo

The dashboard is deployed on Hugging Face Spaces:

**https://abdulwahed-sweden-polaris-chronos.hf.space**

### Run Locally

```bash
cargo build --release
./target/release/polaris server --port 3000
# Open http://localhost:3000
```

<br>

## Quick Start

```bash
cargo build --release
```

Binary: `target/release/polaris`

```bash
# City name
polaris Stockholm

# Palestinian cities
polaris Gaza
polaris Jerusalem
polaris Ramallah
polaris Hebron
polaris Nablus

# City + date
polaris --city "New York" --date 2026-03-20

# City + country
polaris --city "Medina, Saudi Arabia"
polaris --city Medina --country SA

# Auto-detect via IP
polaris --auto --now

# Raw coordinates
polaris --lat 21.4225 --lon 39.8262 --tz Asia/Riyadh

# Strict mode (no projection)
polaris Stockholm --strategy strict

# Debug geocoding
polaris --city Paris --topk 5
```

### CLI Output

```
  gaza — Palestine
  Asia/Gaza (Local Time)
  31.50°N, 34.47°E
```

### Multi-City Disambiguation

When a city name matches locations in multiple countries, Polaris shows a selection:

```
Ambiguous city name: 'Medina'

  Multiple matches found:
    1. Medina, Al Madinah, Saudi Arabia — Saudi Arabia
       Asia/Riyadh (Local Time)
       24.47°N, 39.61°E
    2. Medina, OH, US — United States
       America/New_York (Local Time)
       41.14°N, 81.86°W

  Hint: Try --city "Medina, SA" or --country SA
```

The web UI shows clickable buttons for disambiguation. The API returns HTTP 300 with structured options.

### API Response

```bash
curl http://127.0.0.1:3000/api/resolve?query=Gaza
```

```json
{
  "name": "gaza",
  "lat": 31.5017,
  "lon": 34.4668,
  "tz": "Asia/Gaza",
  "tz_label": "Asia/Gaza (Local Time)",
  "country_code": "PS",
  "country": "Palestine",
  "formatted_coords": "31.50°N, 34.47°E",
  "source": "Built-in",
  "confidence": 0.95
}
```

<br>

## CLI Reference

<table>
<thead>
<tr>
<th align="left">Flag</th>
<th align="left">Description</th>
</tr>
</thead>
<tbody>
<tr><td><code>--city</code></td><td>City name (or use positional argument)</td></tr>
<tr><td><code>--country</code></td><td>ISO 3166-1 alpha-2 hint &mdash; <code>SA</code>, <code>NO</code>, <code>US</code></td></tr>
<tr><td><code>--auto</code> / <code>-a</code></td><td>Auto-detect location via IP geolocation</td></tr>
<tr><td><code>--lat</code> / <code>--lon</code></td><td>Manual coordinates (requires <code>--tz</code>)</td></tr>
<tr><td><code>--date</code> / <code>-d</code></td><td>Date in <code>YYYY-MM-DD</code> format (default: today)</td></tr>
<tr><td><code>--tz</code></td><td>IANA timezone override &mdash; <code>Europe/Oslo</code></td></tr>
<tr><td><code>--strategy</code></td><td><code>projected45</code> (default) or <code>strict</code></td></tr>
<tr><td><code>--now</code></td><td>Show current prayer and countdown to next</td></tr>
<tr><td><code>--show-confidence</code></td><td>Display confidence scores in ASCII timeline</td></tr>
<tr><td><code>--topk</code></td><td>Show top-K Nominatim candidates</td></tr>
<tr><td><code>--offline</code></td><td>Skip network calls; use cache and built-in data only</td></tr>
</tbody>
</table>

<br>

## Web Server & API

```bash
polaris server --port 3000
```

### API Endpoints

| Endpoint | Description |
|----------|-------------|
| `GET /api/resolve?query=stockholm` | Resolve city to coordinates + timezone |
| `GET /api/times?city=stockholm&date=2026-03-01` | Prayer times for a specific date |
| `GET /api/month?city=stockholm&year=2026&month=3` | Full month of prayer times |
| `GET /api/hijri?lat=21.42&lon=39.83&tz=Asia/Riyadh` | Hijri calendar + Ramadan dates |
| `GET /api/cities` | List all 34 built-in cities |

### Fresh Data Guarantee

Every API response includes headers that prevent stale data:

| Header | Value |
|--------|-------|
| `Cache-Control` | `no-store, no-cache, must-revalidate, max-age=0` |
| `Pragma` | `no-cache` |
| `x-polaris-version` | Current version (e.g. `1.0.0`) |

<br>

## Docker & Deployment

### Hugging Face Spaces (Live)

The project includes a `Dockerfile` for deployment to Hugging Face Spaces:

```bash
# Builds automatically when pushed to HF Spaces
git push hf main
```

### Run with Docker Locally

```bash
docker build -t polaris-chronos .
docker run -p 7860:7860 polaris-chronos
# Open http://localhost:7860
```

The Docker image uses a multi-stage build (Rust builder + Debian slim runtime) and runs as a non-root user on port 7860.

<br>

## Architecture

```
src/
  main.rs              CLI entry point (clap)
  lib.rs               Library root & public API
  solar.rs             SPA solar position algorithm (Jean Meeus)
  schedule.rs          Prayer event scheduling & gap strategies
  solver.rs            Solver engine + ASCII timeline renderer
  location/
    mod.rs             Module exports
    types.rs           ResolvedLocation, LocationError, confidence
    resolver.rs        Fallback chain: cache → built-in → Nominatim → IP
    providers.rs       Nominatim geocoder, IP API, 34 city dataset
    cache.rs           File-based location cache (30-day TTL)
  server/
    mod.rs             Axum web server, API routes, CORS
    static_files.rs    Embedded static assets (include_str!)
static/
  index.html           Dashboard (Tailwind CSS + Lucide Icons)
  style.css            Custom properties, animations, SVG dial
  app.js               Client-side calendar, GPS, routing, charts
Dockerfile             Multi-stage build for HF Spaces (port 7860)
```

<details>
<summary><strong>Location Resolution Chain</strong></summary>
<br>

1. **File cache** &mdash; instant, offline
2. **Built-in dataset** &mdash; fuzzy matching across 34 cities (including Palestinian cities)
3. **Nominatim geocoding** &mdash; country filtering, scoring, and interactive disambiguation
4. **IP geolocation** &mdash; fallback

</details>

<details>
<summary><strong>Solar Engine</strong></summary>
<br>

Samples altitude at 1-minute resolution across 24 hours, then applies threshold detection for each prayer event. When a threshold is never crossed, the engine switches to Virtual or Projected mode automatically.

</details>

<details>
<summary><strong>Frontend Stack</strong></summary>
<br>

- **Tailwind CSS** (Play CDN) &mdash; all layout and styling via utility classes
- **Lucide Icons** (CDN) &mdash; professional SVG icon set
- **Inter** body font, **Plus Jakarta Sans** headings, **JetBrains Mono** data, **Readex Pro** Arabic
- Emerald green (`#059669`) brand color, WCAG AAA contrast
- No Bootstrap, no dark mode, no build step

</details>

<br>

## Testing

```bash
# 96 unit tests — solar, schedule, solver, location, Palestine
cargo test

# Global stress test — 30 cities × 3 dates + fuzzy edge cases
cargo build --release && python3 scripts/global_maghrib_test.py
```

<br>

## Design Principles

<table>
<tbody>
<tr>
<td><strong>Physics-first</strong></td>
<td>The sun's position is computed, never approximated or hard-coded</td>
</tr>
<tr>
<td><strong>Transparent</strong></td>
<td>Every output value explains how it was derived</td>
</tr>
<tr>
<td><strong>Universal</strong></td>
<td>Works identically from Mecca (21&deg;N) to Svalbard (78&deg;N) to the South Pole</td>
</tr>
<tr>
<td><strong>Deterministic</strong></td>
<td>Same coordinates + same date = same result, always</td>
</tr>
<tr>
<td><strong>Honest</strong></td>
<td>When precision drops, confidence drops with it</td>
</tr>
</tbody>
</table>

<br>

## License

MIT
