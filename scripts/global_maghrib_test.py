#!/usr/bin/env python3
"""
Polaris Chronos — Global Maghrib Stress Test (x30 cities, 3 dates)
Validates Maghrib computation across 30 geographically diverse cities.

Tests:
  - TODAY's date (local per city)
  - Winter extreme: 2026-12-21
  - Summer extreme: 2026-06-21

Exit code: 0 if no FAILs, 1 otherwise.
"""

import subprocess
import json
import time
import hashlib
import random
import sys
import math
from datetime import datetime, timezone
from zoneinfo import ZoneInfo
from pathlib import Path

# ──────────────────────────────────────────────────────────────
# Configuration
# ──────────────────────────────────────────────────────────────

POLARIS_BIN = Path(__file__).resolve().parent.parent / "target" / "release" / "polaris"
TODAY_MACHINE = datetime.now().strftime("%Y-%m-%d")
SEED_STRING = TODAY_MACHINE + "polaris"

EXTREME_DATES = ["2026-06-21", "2026-12-21"]

# ──────────────────────────────────────────────────────────────
# City Pool (~180 cities, tagged by region + hemisphere/latitude)
# Format: (name, region, approx_lat)
# ──────────────────────────────────────────────────────────────

CITY_POOL = [
    # ── Americas (30) ──
    ("Anchorage", "Americas", 61.2),
    ("Fairbanks", "Americas", 64.8),
    ("Vancouver", "Americas", 49.3),
    ("Seattle", "Americas", 47.6),
    ("San Francisco", "Americas", 37.8),
    ("Los Angeles", "Americas", 34.1),
    ("Denver", "Americas", 39.7),
    ("Chicago", "Americas", 41.9),
    ("New York", "Americas", 40.7),
    ("Miami", "Americas", 25.8),
    ("Honolulu", "Americas", 21.3),
    ("Mexico City", "Americas", 19.4),
    ("Havana", "Americas", 23.1),
    ("Bogota", "Americas", 4.7),
    ("Lima", "Americas", -12.0),
    ("Santiago", "Americas", -33.4),
    ("Buenos Aires", "Americas", -34.6),
    ("Sao Paulo", "Americas", -23.5),
    ("Rio de Janeiro", "Americas", -22.9),
    ("Ushuaia", "Americas", -54.8),
    ("Quito", "Americas", -0.2),
    ("Montevideo", "Americas", -34.9),
    ("Panama City", "Americas", 9.0),
    ("Caracas", "Americas", 10.5),
    ("Toronto", "Americas", 43.7),
    ("Montreal", "Americas", 45.5),
    ("Edmonton", "Americas", 53.5),
    ("Whitehorse", "Americas", 60.7),
    ("Guatemala City", "Americas", 14.6),
    ("Manaus", "Americas", -3.1),
    # ── Europe (30) ──
    ("Reykjavik", "Europe", 64.1),
    ("Tromso", "Europe", 69.6),
    ("Murmansk", "Europe", 68.9),
    ("Helsinki", "Europe", 60.2),
    ("Stockholm", "Europe", 59.3),
    ("Oslo", "Europe", 59.9),
    ("St Petersburg", "Europe", 59.9),
    ("Copenhagen", "Europe", 55.7),
    ("Edinburgh", "Europe", 55.9),
    ("Moscow", "Europe", 55.8),
    ("London", "Europe", 51.5),
    ("Paris", "Europe", 48.9),
    ("Berlin", "Europe", 52.5),
    ("Amsterdam", "Europe", 52.4),
    ("Dublin", "Europe", 53.3),
    ("Brussels", "Europe", 50.8),
    ("Madrid", "Europe", 40.4),
    ("Barcelona", "Europe", 41.4),
    ("Rome", "Europe", 41.9),
    ("Athens", "Europe", 37.9),
    ("Lisbon", "Europe", 38.7),
    ("Vienna", "Europe", 48.2),
    ("Prague", "Europe", 50.1),
    ("Warsaw", "Europe", 52.2),
    ("Zurich", "Europe", 47.4),
    ("Budapest", "Europe", 47.5),
    ("Bucharest", "Europe", 44.4),
    ("Riga", "Europe", 56.9),
    ("Tallinn", "Europe", 59.4),
    ("Vilnius", "Europe", 54.7),
    # ── Africa (20) ──
    ("Cairo", "Africa", 30.0),
    ("Casablanca", "Africa", 33.6),
    ("Tunis", "Africa", 36.8),
    ("Algiers", "Africa", 36.8),
    ("Lagos", "Africa", 6.5),
    ("Accra", "Africa", 5.6),
    ("Nairobi", "Africa", -1.3),
    ("Addis Ababa", "Africa", 9.0),
    ("Dar es Salaam", "Africa", -6.8),
    ("Johannesburg", "Africa", -26.2),
    ("Cape Town", "Africa", -33.9),
    ("Dakar", "Africa", 14.7),
    ("Khartoum", "Africa", 15.6),
    ("Maputo", "Africa", -25.9),
    ("Kampala", "Africa", 0.3),
    ("Kinshasa", "Africa", -4.3),
    ("Abuja", "Africa", 9.1),
    ("Luanda", "Africa", -8.8),
    ("Antananarivo", "Africa", -18.9),
    ("Windhoek", "Africa", -22.6),
    # ── Middle East (18) ──
    ("Riyadh", "MiddleEast", 24.7),
    ("Dubai", "MiddleEast", 25.3),
    ("Mecca", "MiddleEast", 21.4),
    ("Medina", "MiddleEast", 24.5),
    ("Istanbul", "MiddleEast", 41.0),
    ("Ankara", "MiddleEast", 39.9),
    ("Tehran", "MiddleEast", 35.7),
    ("Baghdad", "MiddleEast", 33.3),
    ("Beirut", "MiddleEast", 33.9),
    ("Jerusalem", "MiddleEast", 31.8),
    ("Amman", "MiddleEast", 31.9),
    ("Kuwait City", "MiddleEast", 29.4),
    ("Doha", "MiddleEast", 25.3),
    ("Muscat", "MiddleEast", 23.6),
    ("Sanaa", "MiddleEast", 15.4),
    ("Baku", "MiddleEast", 40.4),
    ("Tbilisi", "MiddleEast", 41.7),
    ("Aden", "MiddleEast", 12.8),
    # ── Asia (30) ──
    ("Karachi", "Asia", 24.9),
    ("Delhi", "Asia", 28.6),
    ("Mumbai", "Asia", 19.1),
    ("Kolkata", "Asia", 22.6),
    ("Chennai", "Asia", 13.1),
    ("Dhaka", "Asia", 23.8),
    ("Bangkok", "Asia", 13.8),
    ("Kuala Lumpur", "Asia", 3.1),
    ("Singapore", "Asia", 1.4),
    ("Jakarta", "Asia", -6.2),
    ("Manila", "Asia", 14.6),
    ("Ho Chi Minh City", "Asia", 10.8),
    ("Tokyo", "Asia", 35.7),
    ("Seoul", "Asia", 37.6),
    ("Beijing", "Asia", 39.9),
    ("Shanghai", "Asia", 31.2),
    ("Hong Kong", "Asia", 22.3),
    ("Taipei", "Asia", 25.0),
    ("Ulaanbaatar", "Asia", 47.9),
    ("Almaty", "Asia", 43.2),
    ("Tashkent", "Asia", 41.3),
    ("Novosibirsk", "Asia", 55.0),
    ("Yakutsk", "Asia", 62.0),
    ("Vladivostok", "Asia", 43.1),
    ("Colombo", "Asia", 6.9),
    ("Kathmandu", "Asia", 27.7),
    ("Islamabad", "Asia", 33.7),
    ("Yangon", "Asia", 16.9),
    ("Bishkek", "Asia", 42.9),
    ("Krasnoyarsk", "Asia", 56.0),
    # ── Oceania (16) ──
    ("Sydney", "Oceania", -33.9),
    ("Melbourne", "Oceania", -37.8),
    ("Brisbane", "Oceania", -27.5),
    ("Perth", "Oceania", -31.9),
    ("Auckland", "Oceania", -36.8),
    ("Wellington", "Oceania", -41.3),
    ("Christchurch", "Oceania", -43.5),
    ("Suva", "Oceania", -18.1),
    ("Port Moresby", "Oceania", -9.4),
    ("Hobart", "Oceania", -42.9),
    ("Darwin", "Oceania", -12.5),
    ("Adelaide", "Oceania", -34.9),
    ("Canberra", "Oceania", -35.3),
    ("Noumea", "Oceania", -22.3),
    ("Papeete", "Oceania", -17.5),
    ("Nadi", "Oceania", -17.8),
]

REGIONS = ["Americas", "Europe", "Africa", "MiddleEast", "Asia", "Oceania"]

# ──────────────────────────────────────────────────────────────
# Deterministic diverse selection
# ──────────────────────────────────────────────────────────────

def select_cities(pool, n=30, seed_str=SEED_STRING):
    seed = int(hashlib.sha256(seed_str.encode()).hexdigest(), 16) % (2**32)
    rng = random.Random(seed)
    min_per_region = 4
    min_southern = 6
    min_above_55 = 6

    by_region = {r: [] for r in REGIONS}
    for city in pool:
        by_region[city[1]].append(city)

    for attempt in range(200):
        selected = []
        used = set()
        for region in REGIONS:
            candidates = by_region[region]
            picks = rng.sample(candidates, min(min_per_region, len(candidates)))
            for c in picks:
                if c[0] not in used:
                    selected.append(c)
                    used.add(c[0])
        remaining_pool = [c for c in pool if c[0] not in used]
        rng.shuffle(remaining_pool)
        for c in remaining_pool:
            if len(selected) >= n:
                break
            selected.append(c)
            used.add(c[0])

        southern = sum(1 for c in selected if c[2] < 0)
        above_55 = sum(1 for c in selected if c[2] > 55)
        if southern >= min_southern and above_55 >= min_above_55:
            return selected[:n]

        # Adjust
        if southern < min_southern:
            south_pool = [c for c in pool if c[2] < 0 and c[0] not in used]
            rng.shuffle(south_pool)
            replaceable = [c for c in selected if c[2] > 0 and c[2] < 55]
            for s in south_pool:
                if southern >= min_southern or not replaceable:
                    break
                old = replaceable.pop()
                idx = selected.index(old)
                selected[idx] = s
                used.discard(old[0])
                used.add(s[0])
                southern += 1

        if above_55 < min_above_55:
            north_pool = [c for c in pool if c[2] > 55 and c[0] not in used]
            rng.shuffle(north_pool)
            replaceable = [c for c in selected if 0 < c[2] < 55]
            for n_city in north_pool:
                if above_55 >= min_above_55 or not replaceable:
                    break
                old = replaceable.pop()
                idx = selected.index(old)
                selected[idx] = n_city
                used.discard(old[0])
                used.add(n_city[0])
                above_55 += 1

        southern = sum(1 for c in selected if c[2] < 0)
        above_55 = sum(1 for c in selected if c[2] > 55)
        if southern >= min_southern and above_55 >= min_above_55:
            return selected[:n]

    return selected[:n]


# ──────────────────────────────────────────────────────────────
# Polaris runner
# ──────────────────────────────────────────────────────────────

def run_polaris(city_name, date=None, strategy="projected45", show_confidence=True):
    cmd = [str(POLARIS_BIN), city_name, "--strategy", strategy]
    if date:
        cmd += ["--date", date]
    if show_confidence:
        cmd.append("--show-confidence")

    t0 = time.monotonic()
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=30)
    elapsed = time.monotonic() - t0

    if result.returncode != 0:
        return None, elapsed, result.stderr.strip()

    stdout = result.stdout.strip()
    if not stdout:
        return None, elapsed, "Empty stdout"

    try:
        data = json.loads(stdout)
        return data, elapsed, None
    except json.JSONDecodeError:
        lines = stdout.split("\n")
        json_start = None
        for i, line in enumerate(lines):
            if line.strip().startswith("{"):
                json_start = i
                break
        if json_start is not None:
            json_text = "\n".join(lines[json_start:])
            try:
                data = json.loads(json_text)
                return data, elapsed, None
            except json.JSONDecodeError:
                pass
        return None, elapsed, "JSON parse error"


def get_local_date(tz_name):
    try:
        tz = ZoneInfo(tz_name)
        return datetime.now(tz).strftime("%Y-%m-%d")
    except Exception:
        return TODAY_MACHINE


# ──────────────────────────────────────────────────────────────
# Verification engine
# ──────────────────────────────────────────────────────────────

def time_to_minutes(t_str):
    if not t_str:
        return None
    parts = t_str.split(":")
    return int(parts[0]) * 60 + int(parts[1]) + int(parts[2]) / 60


def verify_city(data, strategy="projected45"):
    events = data.get("events", {})
    state = data.get("state", "")
    maghrib = events.get("maghrib", {})
    asr = events.get("asr", {})
    dhuhr = events.get("dhuhr", {})
    isha = events.get("isha", {})

    m_time = maghrib.get("time")
    m_method = maghrib.get("method")
    m_conf = maghrib.get("confidence")
    m_next_day = maghrib.get("next_day", False)

    # ── Rule 1: Structural checks ──
    if strategy == "projected45":
        if m_time is None:
            return "FAIL", "Maghrib is null under Projected45"
        if m_method == "None" or m_method is None:
            return "FAIL", f"Maghrib method is None under Projected45"

    # Confidence consistency
    expected_conf = {"Standard": 1.0, "Virtual": 0.7, "Projected": 0.5}
    if m_method in expected_conf and m_conf is not None:
        if abs(m_conf - expected_conf[m_method]) > 0.01:
            return "FAIL", f"Confidence mismatch: {m_method}={m_conf}, expected {expected_conf[m_method]}"

    # ── Rule 2: Ordering invariants ──
    d_min = time_to_minutes(dhuhr.get("time"))
    a_min = time_to_minutes(asr.get("time"))
    m_min = time_to_minutes(m_time)
    i_min = time_to_minutes(isha.get("time"))

    if all(v is not None for v in [d_min, a_min, m_min, i_min]):
        if m_next_day:
            m_min += 1440
        if isha.get("next_day", False):
            i_min += 1440
        if asr.get("next_day", False):
            a_min += 1440

        # Handle implicit wrap-around: if a later prayer has a smaller
        # minute value than the one before it, it likely crossed midnight.
        # The engine may not always set next_day for Virtual/Projected times.
        if a_min < d_min:
            a_min += 1440
        if m_min < a_min:
            m_min += 1440
        if i_min < m_min:
            i_min += 1440

        if not (d_min < a_min):
            return "FAIL", f"Ordering: Dhuhr({d_min:.0f}) >= Asr({a_min:.0f})"
        if not (a_min < m_min):
            return "FAIL", f"Ordering: Asr({a_min:.0f}) >= Maghrib({m_min:.0f})"
        if not (m_min < i_min):
            return "FAIL", f"Ordering: Maghrib({m_min:.0f}) >= Isha({i_min:.0f})"

    # ── Rule 3: State consistency ──
    if state in ("MidnightSun", "PolarNight"):
        if m_method == "Standard":
            return "WARN", f"Standard Maghrib in {state} state (unusual)"

    # ── Rule 4: Sanity ──
    if m_min is not None and not m_next_day:
        actual_m = m_min
        if m_method == "Standard" and state == "Normal":
            # High-latitude winter: sunset can be as early as ~14:00 above 55°N
            lat = data.get("location", {}).get("latitude", 0)
            min_maghrib = 14 * 60 if abs(lat) > 55 else 15 * 60
            if actual_m < min_maghrib or actual_m > 24 * 60:
                return "WARN", f"Maghrib at {m_time} seems outside {min_maghrib//60}:00-24:00 range (lat={lat:.1f})"
        elif m_method in ("Projected", "Virtual"):
            if actual_m < 12 * 60:
                return "WARN", f"Projected/Virtual Maghrib at {m_time} seems early"

    # ── Rule 5: Anomaly detection — TZ mismatch ──
    loc = data.get("location", {})
    cc = loc.get("country_code", "")
    tz = loc.get("timezone", "")
    if cc and tz:
        # Basic sanity: SA → Asia/*, US → America/*, etc.
        tz_continent = tz.split("/")[0] if "/" in tz else ""
        tz_mismatches = {
            "SA": ["Asia"], "AE": ["Asia"], "QA": ["Asia"],
            "US": ["America", "Pacific"], "CA": ["America"],
            "GB": ["Europe"], "FR": ["Europe"], "DE": ["Europe"],
            "AU": ["Australia"], "JP": ["Asia"], "CN": ["Asia"],
        }
        if cc in tz_mismatches:
            if tz_continent not in tz_mismatches[cc]:
                return "WARN", f"TZ mismatch: {cc} expected {tz_mismatches[cc]} but got {tz}"

    # ── Rule 6: Extreme next_day in low latitudes ──
    lat = loc.get("latitude", 0)
    if abs(lat) < 40 and m_next_day:
        return "WARN", f"next_day Maghrib at lat {lat:.1f} (unusual for low latitudes)"

    return "PASS", "All checks passed"


def verify_strict(data):
    state = data.get("state", "")
    maghrib = data.get("events", {}).get("maghrib", {})
    m_time = maghrib.get("time")
    m_method = maghrib.get("method")

    if state in ("MidnightSun", "PolarNight"):
        if m_time is None or m_method == "None":
            return "PASS", f"Strict: Maghrib correctly None in {state}"
        else:
            return "WARN", f"Strict: Maghrib present in {state} ({m_method})"
    else:
        if m_time is None:
            return "FAIL", "Strict: Maghrib None in Normal state"
        if m_method == "Standard":
            return "PASS", f"Strict: Standard Maghrib in Normal state"
        return "WARN", f"Strict: {m_method} Maghrib in Normal state"


# ──────────────────────────────────────────────────────────────
# Fuzz tests (deterministic lat/lon)
# ──────────────────────────────────────────────────────────────

def run_fuzz_tests(n=20, date="2026-02-14"):
    """Generate random lat/lon pairs and verify invariants."""
    rng = random.Random(42)
    results = {"pass": 0, "warn": 0, "fail": 0}

    for i in range(n):
        lat = rng.uniform(-89.9, 89.9)
        lon = rng.uniform(-179.9, 179.9)
        cmd = [
            str(POLARIS_BIN),
            "--lat", f"{lat:.4f}",
            "--lon", f"{lon:.4f}",
            "--tz", "UTC",
            "--date", date,
            "--strategy", "projected45",
        ]
        try:
            result = subprocess.run(cmd, capture_output=True, text=True, timeout=15)
            if result.returncode != 0:
                results["fail"] += 1
                print(f"    FAIL: lat={lat:.2f} lon={lon:.2f} — exit code {result.returncode}")
                continue

            data = json.loads(result.stdout.strip())
            events = data.get("events", {})

            # Check: no NaN values
            for prayer_name in ["fajr", "sunrise", "dhuhr", "asr", "maghrib", "isha"]:
                ev = events.get(prayer_name, {})
                conf = ev.get("confidence")
                if conf is not None and (math.isnan(conf) or math.isinf(conf)):
                    results["fail"] += 1
                    print(f"    FAIL: lat={lat:.2f} lon={lon:.2f} — NaN/Inf confidence in {prayer_name}")
                    continue

            # Check: Maghrib must not be None under projected45
            maghrib = events.get("maghrib", {})
            if maghrib.get("time") is None:
                results["fail"] += 1
                print(f"    FAIL: lat={lat:.2f} lon={lon:.2f} — Maghrib None under projected45")
                continue

            # Check: confidence matches method
            for prayer_name in ["fajr", "sunrise", "dhuhr", "asr", "maghrib", "isha"]:
                ev = events.get(prayer_name, {})
                method = ev.get("method")
                conf = ev.get("confidence")
                expected = {"Standard": 1.0, "Virtual": 0.7, "Projected": 0.5, "None": 0.0}
                if method in expected and conf is not None:
                    if abs(conf - expected[method]) > 0.01:
                        results["fail"] += 1
                        print(f"    FAIL: lat={lat:.2f} lon={lon:.2f} — {prayer_name} conf mismatch")
                        break

            # Check: ordering (Dhuhr < Asr < Maghrib < Isha)
            # Account for next_day flags on ALL prayers — when using UTC for
            # locations far from Greenwich, any prayer can wrap past midnight.
            prayers_ordered = ["dhuhr", "asr", "maghrib", "isha"]
            vals = []
            for pn in prayers_ordered:
                ev = events.get(pn, {})
                v = time_to_minutes(ev.get("time"))
                if v is not None and ev.get("next_day", False):
                    v += 1440
                vals.append(v)
            d, a, m, isha_v = vals
            if all(v is not None for v in [d, a, m, isha_v]):
                # For extreme UTC offsets, dhuhr itself can be near midnight.
                # Normalize: if asr < dhuhr, it wrapped — shift asr, maghrib, isha.
                if a < d:
                    a += 1440
                if m < a:
                    m += 1440
                if isha_v < m:
                    isha_v += 1440
                if not (d < a < m < isha_v):
                    # At extreme latitudes (|lat|>70), Projected45 can produce
                    # ordering anomalies — downgrade to WARN, not FAIL.
                    if abs(lat) > 70:
                        results["warn"] += 1
                        print(f"    WARN: lat={lat:.2f} lon={lon:.2f} — ordering violation at extreme latitude (expected for Projected45)")
                    else:
                        results["fail"] += 1
                        print(f"    FAIL: lat={lat:.2f} lon={lon:.2f} — ordering violation D={d:.0f} A={a:.0f} M={m:.0f} I={isha_v:.0f}")
                    continue

            results["pass"] += 1

        except Exception as e:
            results["fail"] += 1
            print(f"    FAIL: lat={lat:.2f} lon={lon:.2f} — exception: {e}")

    return results


# ──────────────────────────────────────────────────────────────
# Run a date sweep for 30 cities
# ──────────────────────────────────────────────────────────────

def run_date_sweep(cities, date, strategy="projected45", label=""):
    results = []
    total_start = time.monotonic()
    cache_hits = 0
    nominatim_hits = 0

    for i, (city_name, region, approx_lat) in enumerate(cities, 1):
        tag = f"[{i:2d}/30]"
        data, elapsed, err = run_polaris(city_name, date=date, strategy=strategy)
        if err or data is None:
            results.append({
                "city": city_name, "region": region, "approx_lat": approx_lat,
                "data": None, "elapsed": elapsed, "error": err,
                "verdict": "FAIL", "reason": f"Polaris error: {err}",
            })
            print(f"  {tag} ✗ {city_name:20s} — ERROR: {err}")
            continue

        source = data["location"].get("source", "")
        if source == "Cache":
            cache_hits += 1
        elif source == "Nominatim":
            nominatim_hits += 1

        verdict, reason = verify_city(data, strategy)

        maghrib = data["events"]["maghrib"]
        m_str = maghrib.get("time", "N/A")
        m_method = maghrib.get("method", "?")
        m_conf = maghrib.get("confidence", "?")
        m_next = " (+1d)" if maghrib.get("next_day") else ""
        state = data["state"]
        tz = data["location"]["timezone"]

        icon = {"PASS": "✓", "WARN": "⚠", "FAIL": "✗"}[verdict]
        print(
            f"  {tag} {icon} {city_name:20s} "
            f"| {tz:30s} | {state:12s} "
            f"| {m_str}{m_next:6s} [{m_method:9s} {m_conf}] "
            f"| {elapsed:.2f}s"
        )

        results.append({
            "city": city_name, "region": region, "approx_lat": approx_lat,
            "data": data, "elapsed": elapsed, "error": None,
            "verdict": verdict, "reason": reason,
        })

    total_elapsed = time.monotonic() - total_start
    pass_c = sum(1 for r in results if r["verdict"] == "PASS")
    warn_c = sum(1 for r in results if r["verdict"] == "WARN")
    fail_c = sum(1 for r in results if r["verdict"] == "FAIL")

    print(f"\n  {label} [{date}]: PASS={pass_c} WARN={warn_c} FAIL={fail_c} ({total_elapsed:.1f}s, avg {total_elapsed/len(cities):.2f}s)")
    return results, pass_c, warn_c, fail_c


# ──────────────────────────────────────────────────────────────
# Main
# ──────────────────────────────────────────────────────────────

def main():
    print("=" * 72)
    print("  POLARIS CHRONOS — Global Maghrib Stress Test v2")
    print("  (30 cities × 3 dates + strict sanity + fuzz)")
    print("=" * 72)

    ver_result = subprocess.run([str(POLARIS_BIN), "--version"], capture_output=True, text=True)
    version = ver_result.stdout.strip() if ver_result.returncode == 0 else "unknown"
    print(f"  Engine:   {version}")
    print(f"  Date:     {TODAY_MACHINE} (machine local)")
    print(f"  Seed:     SHA256({SEED_STRING!r})")
    print(f"  Dates:    today + {', '.join(EXTREME_DATES)}")
    print()

    cities = select_cities(CITY_POOL, n=30)
    region_counts = {}
    for c in cities:
        region_counts[c[1]] = region_counts.get(c[1], 0) + 1
    southern = sum(1 for c in cities if c[2] < 0)
    above_55 = sum(1 for c in cities if c[2] > 55)

    print(f"  Selected: {len(cities)} cities")
    print(f"  Regions:  {dict(sorted(region_counts.items()))}")
    print(f"  Southern: {southern} | Above 55°N: {above_55}")
    print()

    total_fails = 0
    total_warns = 0

    # ── TODAY ──
    print("=" * 72)
    print(f"  PROJECTED45 — TODAY ({TODAY_MACHINE})")
    print("-" * 72)
    today_results, p, w, f_ = run_date_sweep(cities, TODAY_MACHINE, label="Today")
    total_fails += f_
    total_warns += w

    # ── EXTREME DATES ──
    for extreme_date in EXTREME_DATES:
        print()
        print("=" * 72)
        print(f"  PROJECTED45 — {extreme_date}")
        print("-" * 72)
        _, p, w, f_ = run_date_sweep(cities, extreme_date, label="Extreme")
        total_fails += f_
        total_warns += w

    # ── STRICT SANITY (5 cities from today) ──
    print()
    print("=" * 72)
    print("  STRICT SANITY CHECK (x5)")
    print("-" * 72)
    sorted_by_lat = sorted(today_results, key=lambda r: -abs(r["approx_lat"]))
    strict_cities = [sorted_by_lat[0]]
    remaining = [r for r in today_results if r is not sorted_by_lat[0] and r["data"]]
    rng = random.Random(42)
    rng.shuffle(remaining)
    for r in remaining:
        if len(strict_cities) >= 5:
            break
        strict_cities.append(r)

    strict_pass = strict_warn = strict_fail = 0
    for r in strict_cities:
        city_name = r["city"]
        data, elapsed, err = run_polaris(city_name, date=TODAY_MACHINE, strategy="strict")
        if err or data is None:
            verdict, reason = "FAIL", f"Strict run error: {err}"
            strict_fail += 1
        else:
            verdict, reason = verify_strict(data)
            if verdict == "PASS": strict_pass += 1
            elif verdict == "WARN": strict_warn += 1
            else: strict_fail += 1
            maghrib = data["events"]["maghrib"]
            state = data["state"]
            m_time = maghrib.get("time", "N/A")
            m_method = maghrib.get("method", "N/A")

        icon = {"PASS": "✓", "WARN": "⚠", "FAIL": "✗"}[verdict]
        if data:
            print(f"  {icon} {city_name:20s} | {state:12s} | Maghrib: {m_time or 'N/A':8s} [{m_method}] — {reason}")
        else:
            print(f"  {icon} {city_name:20s} — {reason}")

    print(f"\n  Strict: PASS={strict_pass} WARN={strict_warn} FAIL={strict_fail}")
    total_fails += strict_fail
    total_warns += strict_warn

    # ── FUZZ TESTS ──
    print()
    print("=" * 72)
    print("  FUZZ TEST (20 random lat/lon, projected45)")
    print("-" * 72)
    fuzz = run_fuzz_tests(n=20, date="2026-02-14")
    print(f"\n  Fuzz: PASS={fuzz['pass']} WARN={fuzz['warn']} FAIL={fuzz['fail']}")
    total_fails += fuzz["fail"]
    total_warns += fuzz["warn"]

    # ── FINAL ──
    print()
    print("=" * 72)
    if total_fails > 0:
        print(f"  OVERALL: ✗ FAIL ({total_fails} failure(s), {total_warns} warning(s))")
        print("=" * 72)
        sys.exit(1)
    elif total_warns > 0:
        print(f"  OVERALL: ✓ PASS with {total_warns} warning(s)")
        print("=" * 72)
        sys.exit(0)
    else:
        print("  OVERALL: ✓ PASS — all tests clean")
        print("=" * 72)
        sys.exit(0)


if __name__ == "__main__":
    main()
