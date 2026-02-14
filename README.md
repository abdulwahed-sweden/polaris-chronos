# Polaris 🌌
### The Universal Prayer Time Engine

> 🌐 Languages: [English](README.md) | [العربية](README_AR.md)

**Polaris** is a high-precision astronomical engine written in Rust, designed to solve prayer time calculations for **all locations on Earth**, including extreme latitudes (Midnight Sun & Polar Night).

---

## 🌍 Why Two Documentations?

Different audiences require different explanations:

- **Developers (English):** Need installation, API usage, and technical clarity.
- **Arabic users / researchers:** Need conceptual understanding and trust — *"How was this calculated?"*

👉 Therefore:
- `README.md` → Technical (this file)
- `README_AR.md` → Conceptual explanation (Q&A format)

---

## 🚀 Why Polaris?

| Feature | Traditional Libraries | Polaris Engine |
| :--- | :--- | :--- |
| **Polar Night** | Fails / Returns Error | **Virtual Schedule** (Wave-based) |
| **Midnight Sun** | Missing Maghrib/Isha | **Adaptive Projection** |
| **Transparency** | Hidden logic | **Explicit Method Labels** |
| **Confidence** | Unknown | **Scored (1.0 → 0.5)** |
| **Architecture** | Static formulas | **Dynamic solar simulation** |

---

## 🧠 Core Idea

Polaris treats the sun not as a "visible disk", but as a **continuous angular motion (sine wave)**.

Even when:
- the sun never sets ☀️
- or never rises 🌑

👉 the system still computes a **complete, consistent daily schedule**

---

## ⚙️ Calculation Modes

| Mode | Description |
|------|------------|
| **Standard** | Real astronomical events (sunrise/sunset exist) |
| **Virtual** | Derived from solar wave (no visible twilight) |
| **Projected** | Borrowed duration from moderate latitude (adaptive model) |

Each result includes:

```
time + method + confidence
```

---

## 📦 Installation

```bash
cargo build --release
```

The binary is at `target/release/polaris`.

---

## 📦 Usage

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

---

## 🔧 CLI Flags

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

---

## 📊 Example Output

```
Maghrib: 21:24 [P] (0.5)
Reason: Sun does not set — projected from moderate latitude
```

---

## 🏗️ Architecture

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

---

## 🧪 Testing

```bash
# Rust unit tests (79 tests)
cargo test

# Global stress test (30 cities x 3 dates + fuzz)
cargo build --release
python3 scripts/global_maghrib_test.py
```

---

## 🔬 Design Principles

- **Physics-first:** Never fake astronomical events
- **Transparent:** Every value explains how it was computed
- **Universal:** Works at any latitude
- **Deterministic:** Same input → same output

---

## 📄 License

MIT
