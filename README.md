# Polaris Chronos

**Universal Prayer Time Engine** — solves high-latitude calculations using adaptive projection and angular solar dynamics.

## Features

- SPA-based solar position engine (Jean Meeus algorithm)
- Virtual Horizon logic for twilight events
- Adaptive Compensation Engine (Projected45 strategy)
- Polar Night and Midnight Sun detection
- Smart city resolution with Nominatim disambiguation
- Built-in dataset for 30+ major cities with fuzzy matching
- File-based location cache with 30-day TTL
- IP-based auto-detection fallback
- Confidence scoring per prayer event (Standard / Virtual / Projected)
- JSON output to stdout, ASCII timeline to stderr

## Installation

```bash
cargo build --release
```

The binary is at `target/release/polaris`.

## Usage

```bash
# By city name (positional)
polaris Stockholm

# Named city flag
polaris --city "New York" --date 2026-03-20

# Comma-separated with country
polaris --city "Medina, Saudi Arabia"

# Country hint (ISO alpha-2)
polaris --city Medina --country SA

# Auto-detect via IP
polaris --auto --now

# Raw coordinates
polaris --lat 21.4225 --lon 39.8262 --tz Asia/Riyadh

# Strict mode (no projection for polar gaps)
polaris Stockholm --strategy strict

# Debug: show top-K Nominatim candidates
polaris --city Paris --topk 5
```

## CLI Flags

| Flag | Description |
|------|-------------|
| `--city` | City name |
| `--auto` / `-a` | Auto-detect location via IP |
| `--lat` / `--lon` | Manual coordinates |
| `--date` / `-d` | Date (YYYY-MM-DD), defaults to today |
| `--tz` | IANA timezone override |
| `--strategy` | `projected45` (default) or `strict` |
| `--country` | ISO 3166-1 alpha-2 country hint |
| `--topk` | Show top-K Nominatim candidates |
| `--now` | Show current prayer and countdown |
| `--show-confidence` | Display confidence in ASCII timeline |
| `--offline` | Skip network calls, use cache/built-in only |

## Architecture

```
src/
  main.rs          CLI entry point (clap)
  lib.rs           Library root
  solar.rs         SPA solar position algorithm
  schedule.rs      Prayer scheduling & gap strategies
  solver.rs        Solver + ASCII timeline renderer
  location/
    mod.rs         Module exports
    types.rs       Core types (ResolvedLocation, LocationError)
    resolver.rs    Fallback chain orchestrator
    providers.rs   Nominatim, IP API, built-in dataset
    cache.rs       File-based location cache
scripts/
  global_maghrib_test.py   30-city x 3-date stress test
```

## Testing

```bash
# Rust unit tests (79 tests)
cargo test

# Global stress test (30 cities x 3 dates + fuzz)
cargo build --release
python3 scripts/global_maghrib_test.py
```

## License

MIT
