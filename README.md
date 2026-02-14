<h1 align="center">
  Polaris
</h1>

<p align="center">
  <strong>The Universal Prayer Time Engine</strong>
</p>

<p align="center">
  <a href="#installation"><img src="https://img.shields.io/badge/Rust-2021_Edition-DEA584?logo=rust&logoColor=white" alt="Rust"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-blue.svg" alt="MIT License"></a>
  <a href="#testing"><img src="https://img.shields.io/badge/Tests-79_passing-brightgreen" alt="Tests"></a>
  <a href="#"><img src="https://img.shields.io/badge/Version-0.5.0-purple" alt="Version"></a>
</p>

<p align="center">
  High-precision solar position engine written in Rust.<br>
  Computes prayer times for every location on Earth — including polar regions.
</p>

<p align="center">
  <a href="README.md"><strong>English</strong></a>&nbsp;&nbsp;&nbsp;|&nbsp;&nbsp;&nbsp;<a href="README_AR.md"><strong>العربية</strong></a>
</p>

<br>

<table>
<tr>
<td>

Most prayer time libraries break above 65°N. The sun doesn't set, so there is no Maghrib. The sun doesn't rise, so there is no Fajr.

**Polaris doesn't break.** It models the sun as a continuous angular wave and produces a complete, transparent schedule — everywhere, every day, every edge case.

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
<td>Polar Night — no sunrise</td>
<td>Returns error or blank</td>
<td><strong>Complete virtual schedule</strong></td>
</tr>
<tr>
<td>Midnight Sun — no sunset</td>
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

The engine computes the sun's altitude at every minute of the day using the **Solar Position Algorithm** (Jean Meeus). From this continuous wave, it derives prayer events — even when classical threshold crossings don't exist.

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
<td>Duration borrowed from adaptive reference latitude (~45°–55°)</td>
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

## Real Output &mdash; Tromsø, Norway &mdash; June 21, 2026

> **Midnight Sun.** The sun never sets. Minimum solar altitude: **+3.1°**

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
<td>Ref. latitude 54.7°</td>
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
<td>Ref. latitude 54.7°</td>
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
- **Sunrise & Maghrib** &mdash; projected from reference latitude 54.7° (no real horizon crossing).
- **Fajr & Isha** &mdash; derived from the solar wave's virtual nadir (twilight angle never reached).

Polaris detected `MidnightSun` state and applied `Projected45` strategy automatically.

</details>

<br>

## Quick Start

```bash
cargo build --release
```

Binary: `target/release/polaris`

```bash
# City name
polaris Stockholm

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
    providers.rs       Nominatim geocoder, IP API, 30+ city dataset
    cache.rs           File-based location cache (30-day TTL)
scripts/
  global_maghrib_test.py   Stress test: 30 cities × 3 dates + fuzz
```

<details>
<summary><strong>Location Resolution Chain</strong></summary>
<br>

1. **File cache** &mdash; instant, offline
2. **Built-in dataset** &mdash; fuzzy matching across 30+ major cities
3. **Nominatim geocoding** &mdash; country filtering and disambiguation
4. **IP geolocation** &mdash; fallback

</details>

<details>
<summary><strong>Solar Engine</strong></summary>
<br>

Samples altitude at 1-minute resolution across 24 hours, then applies threshold detection for each prayer event. When a threshold is never crossed, the engine switches to Virtual or Projected mode automatically.

</details>

<br>

## Testing

```bash
# 79 unit tests — solar, schedule, solver, location
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
<td>Works identically from Mecca (21°N) to Svalbard (78°N) to the South Pole</td>
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
