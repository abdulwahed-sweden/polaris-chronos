<p align="center">
  <h1 align="center">Polaris 🌌</h1>
  <p align="center"><strong>The Universal Prayer Time Engine</strong></p>
  <p align="center">
    High-precision solar position engine written in Rust.<br/>
    Solves prayer time calculations for <em>every location on Earth</em> — including polar regions.
  </p>
  <p align="center">
    <a href="README.md">English</a> · <a href="README_AR.md">العربية</a>
  </p>
</p>

---

Most prayer time libraries break above 65°N. The sun doesn't set, so there is no Maghrib. The sun doesn't rise, so there is no Fajr. **Polaris doesn't break.** It models the sun as a continuous angular wave and produces a complete, transparent schedule — everywhere, every day, every edge case.

---

## Why Polaris?

| Problem | Traditional Libraries | Polaris |
|:--------|:----------------------|:--------|
| Polar Night (no sunrise) | Returns error or blank | Complete virtual schedule |
| Midnight Sun (no sunset) | Missing Maghrib / Isha | Adaptive projection from reference latitude |
| Transparency | Black-box output | Every time labeled with its method |
| Confidence | No indication | Scored per event (1.0 = astronomical, 0.5 = projected) |
| Architecture | Static angle formulas | SPA-based solar simulation with wave analysis |

---

## How It Works

The engine computes the sun's altitude at every minute of the day using the **Solar Position Algorithm** (Jean Meeus). From this continuous wave, it derives prayer events — even when classical threshold crossings don't exist.

Three computation modes, applied automatically:

| Mode | When | What happens |
|:-----|:-----|:-------------|
| **Standard** | Sun crosses the required altitude | Direct astronomical calculation. Confidence: `1.0` |
| **Virtual** | Twilight angle never reached (e.g., Fajr/Isha in polar summer) | Derived from wave nadir/peak timing. Confidence: `0.7` |
| **Projected** | Sunrise or sunset doesn't occur at all | Duration borrowed from an adaptive reference latitude (~45°–55°). Confidence: `0.5` |

Every result carries three fields:

```
time  +  method  +  confidence
```

No hidden logic. No silent fallbacks.

---

## Real Output: Tromsø, Norway — June 21, 2026

**Midnight Sun.** The sun never sets. Minimum solar altitude: +3.1°.

```
polaris Tromso --date 2026-06-21
```

| Prayer | Time | Method | Confidence | Note |
|:-------|:-----|:-------|:-----------|:-----|
| Fajr | 00:46 (+1d) | Virtual | 0.70 | Derived from wave nadir |
| Sunrise | 04:07 | Projected | 0.50 | Ref. latitude 54.7° |
| Dhuhr | 12:46 | Standard | 1.00 | Solar noon |
| Asr | 17:57 | Standard | 1.00 | Shadow-length ratio |
| Maghrib | 21:24 | Projected | 0.50 | Ref. latitude 54.7° |
| Isha | 00:46 (+1d) | Virtual | 0.70 | Derived from wave nadir |

**What happened:** The sun stayed above the horizon for 24 hours. There was no physical sunset or sunrise event. Polaris detected `MidnightSun` state, applied the `Projected45` strategy for Sunrise/Maghrib, and computed Fajr/Isha from the solar wave's virtual nadir. Dhuhr and Asr were calculated normally — the sun still reaches its peak and casts shadows.

---

## Installation

```bash
cargo build --release
```

Binary: `target/release/polaris`

---

## Usage

```bash
# City name (positional)
polaris Stockholm

# Named flag with date
polaris --city "New York" --date 2026-03-20

# City with country (comma syntax)
polaris --city "Medina, Saudi Arabia"

# Country hint (ISO alpha-2)
polaris --city Medina --country SA

# Auto-detect location via IP
polaris --auto --now

# Raw coordinates
polaris --lat 21.4225 --lon 39.8262 --tz Asia/Riyadh

# Strict mode — no projection, gaps shown as missing
polaris Stockholm --strategy strict

# Debug: show top-K geocoding candidates
polaris --city Paris --topk 5
```

---

## CLI Reference

| Flag | Description |
|:-----|:------------|
| `--city` | City name (or use positional argument) |
| `--country` | ISO 3166-1 alpha-2 hint (e.g. `SA`, `NO`, `US`) |
| `--auto` / `-a` | Auto-detect location via IP geolocation |
| `--lat` / `--lon` | Manual coordinates (requires `--tz`) |
| `--date` / `-d` | Date in `YYYY-MM-DD` format (default: today) |
| `--tz` | IANA timezone override (e.g. `Europe/Oslo`) |
| `--strategy` | `projected45` (default) or `strict` |
| `--now` | Show current prayer and countdown to next |
| `--show-confidence` | Display confidence scores in ASCII timeline |
| `--topk` | Show top-K Nominatim candidates |
| `--offline` | Skip network calls; use cache and built-in data only |

---

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

**Location resolution** follows a priority chain:
1. File cache (instant, offline)
2. Built-in dataset with fuzzy matching (30+ major cities)
3. Nominatim geocoding with country filtering and disambiguation
4. IP-based geolocation (fallback)

**Solar engine** samples altitude at 1-minute resolution across 24 hours, then applies threshold detection for each prayer event. When a threshold is never crossed, the engine switches to Virtual or Projected mode automatically.

---

## Testing

```bash
# 79 unit tests — solar, schedule, solver, location
cargo test

# Global stress test — 30 cities × 3 dates + fuzzy edge cases
cargo build --release && python3 scripts/global_maghrib_test.py
```

---

## Design Principles

- **Physics-first** — The sun's position is computed, never approximated or hard-coded
- **Transparent** — Every output value explains how it was derived
- **Universal** — Works identically from Mecca (21°N) to Svalbard (78°N) to the South Pole
- **Deterministic** — Same coordinates + same date = same result, always
- **Honest** — When precision drops, confidence drops with it

---

## License

MIT
