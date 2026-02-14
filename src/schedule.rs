//! Prayer time schedule computation using virtual horizon logic.
//!
//! Core rule: NEVER fake a physical event. If the sun does not cross the
//! horizon, sunrise and sunset are None. Virtual alternatives are provided
//! separately with explicit method labels.

use crate::solar::{self, AltitudeSample, HORIZON_ANGLE};
use chrono::NaiveDate;
use serde::Serialize;
use std::f64::consts::PI;

const DEG: f64 = PI / 180.0;

const FAJR_ANGLE: f64 = -18.0;  // Astronomical twilight (Muslim World League)
const ISHA_ANGLE: f64 = -17.0;  // Isha twilight angle

/// Strategy for handling missing events in polar states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum GapStrategy {
    /// Returns None for missing events (science mode).
    Strict,
    /// Projects durations from 45° latitude (user mode).
    Projected45,
}

impl Default for GapStrategy {
    fn default() -> Self { Self::Projected45 }
}

impl std::fmt::Display for GapStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GapStrategy::Strict => write!(f, "Strict"),
            GapStrategy::Projected45 => write!(f, "Projected45"),
        }
    }
}

/// How a prayer event was determined.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum EventMethod {
    /// Real horizon crossing or standard angular formula.
    Standard,
    /// Derived from angular dynamics (no physical horizon crossing).
    Virtual,
    /// Projected from an adaptive reference latitude (Aqrab al-Bilad).
    Projected,
    /// Event does not exist physically for this day state.
    None,
}

/// A single prayer event: optional time + derivation method.
#[derive(Debug, Clone, Serialize)]
pub struct PrayerEvent {
    /// Local time string (HH:MM:SS) or null if event doesn't exist.
    pub time: Option<String>,
    /// How this time was derived.
    pub method: EventMethod,
    /// Confidence score: 1.0 (real), 0.7 (virtual), 0.5 (projected), 0.0 (none).
    pub confidence: f32,
    /// Projection note (only set for Projected/special events).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// True if this event's local time falls on the next calendar day.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub next_day: bool,
}

impl PrayerEvent {
    fn standard(secs: f64) -> Self {
        Self { time: Some(solar::seconds_to_hms(secs)), method: EventMethod::Standard, confidence: 1.0, note: None, next_day: false }
    }

    fn virtual_event(secs: f64) -> Self {
        Self { time: Some(solar::seconds_to_hms(secs)), method: EventMethod::Virtual, confidence: 0.7, note: None, next_day: false }
    }

    fn none() -> Self {
        Self { time: Option::None, method: EventMethod::None, confidence: 0.0, note: None, next_day: false }
    }

    fn projected(secs: f64, note: &str) -> Self {
        Self {
            time: Some(solar::seconds_to_hms(secs)),
            method: EventMethod::Projected,
            confidence: 0.5,
            note: Some(note.to_string()),
            next_day: false,
        }
    }

    /// Extract seconds for ordering validation (returns 0 for None events).
    pub fn seconds_or(&self, default: f64) -> f64 {
        self.time.as_ref().map(|t| hms_to_seconds(t)).unwrap_or(default)
    }
}

/// Parse HH:MM:SS back to seconds.
fn hms_to_seconds(hms: &str) -> f64 {
    let parts: Vec<&str> = hms.split(':').collect();
    if parts.len() != 3 { return 0.0; }
    let h: f64 = parts[0].parse().unwrap_or(0.0);
    let m: f64 = parts[1].parse().unwrap_or(0.0);
    let s: f64 = parts[2].parse().unwrap_or(0.0);
    h * 3600.0 + m * 60.0 + s
}

/// The state of the solar day.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum DayState {
    /// Sun rises and sets normally.
    Normal,
    /// Sun never sets (altitude always > HORIZON_ANGLE).
    MidnightSun,
    /// Sun never rises (altitude always < HORIZON_ANGLE).
    PolarNight,
}

impl std::fmt::Display for DayState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DayState::Normal => write!(f, "Normal"),
            DayState::MidnightSun => write!(f, "MidnightSun"),
            DayState::PolarNight => write!(f, "PolarNight"),
        }
    }
}

/// Complete prayer schedule with method metadata.
#[derive(Debug, Clone, Serialize)]
pub struct Schedule {
    pub state: DayState,
    pub events: Events,
    pub solar: SolarInfo,
}

#[derive(Debug, Clone, Serialize)]
pub struct Events {
    pub fajr: PrayerEvent,
    pub sunrise: PrayerEvent,
    pub dhuhr: PrayerEvent,
    pub asr: PrayerEvent,
    pub maghrib: PrayerEvent,
    pub isha: PrayerEvent,
}

#[derive(Debug, Clone, Serialize)]
pub struct SolarInfo {
    pub max_altitude: f64,
    pub min_altitude: f64,
    pub peak_utc: String,
    pub nadir_utc: String,
}

/// Determine the DayState from a day scan.
pub fn classify_day(samples: &[AltitudeSample]) -> DayState {
    let has_above = samples.iter().any(|s| s.altitude > HORIZON_ANGLE);
    let has_below = samples.iter().any(|s| s.altitude < HORIZON_ANGLE);
    match (has_above, has_below) {
        (true, true) => DayState::Normal,
        (true, false) => DayState::MidnightSun,
        (false, _) => DayState::PolarNight,
    }
}

// ─── Asr computation ──────────────────────────────────────────────

/// Geometric Asr altitude using zenith formulation.
///
/// The shadow formula: shadow_asr = shadow_noon + object_height
/// In angular terms: tan(z_asr) = 1 + tan(z_noon)
/// Therefore: z_asr = atan(1 + tan(z_noon))
///            alt_asr = 90° - z_asr
///
/// This is equivalent to: alt_asr = atan(1 / (1 + tan(z_noon)))
/// since atan(x) + atan(1/x) = 90° for x > 0.
#[cfg(test)]
fn geometric_asr_altitude(peak_altitude: f64) -> f64 {
    let z_noon_rad = (90.0 - peak_altitude) * DEG;
    let tan_z_asr = 1.0 + z_noon_rad.tan();
    if tan_z_asr <= 0.0 { return 0.0; }
    let z_asr_rad = tan_z_asr.atan();
    (90.0 - z_asr_rad / DEG).max(0.0)
}

/// Standard Asr altitude — equivalent formulation via inverse tangent.
/// alt_asr = atan(1 / (1 + tan(90° - peak)))
fn standard_asr_altitude(peak_altitude: f64) -> f64 {
    let z_noon_rad = (90.0 - peak_altitude) * DEG;
    let denom = 1.0 + z_noon_rad.tan();
    if denom <= 0.0 { return 0.0; }
    (1.0 / denom).atan() / DEG
}

/// Virtual Asr for polar conditions using angular descent from peak.
///
/// In polar night, we compute Asr as the point where the sun has descended
/// by the same *fraction* of its arc that Asr represents in a standard day.
/// A standard Asr at ~45° peak occurs at ~70% of the afternoon. We use
/// the geometric ratio: asr_norm = asr_altitude / peak_altitude (from normal).
///
/// For a reference peak of 55° (typical Mecca):
///   standard asr alt ≈ 31.7°, ratio ≈ 0.576
///   So Asr occurs when the wave has descended to 57.6% of its amplitude.
fn virtual_asr_seconds(
    samples: &[AltitudeSample],
    peak: &AltitudeSample,
    nadir: &AltitudeSample,
) -> f64 {
    // Reference ratio derived from a 55° peak day (Mecca baseline)
    let reference_peak = 55.0;
    let reference_asr = standard_asr_altitude(reference_peak);
    let asr_ratio = reference_asr / reference_peak; // ~0.576

    // Target altitude on the wave = nadir + (peak - nadir) * asr_ratio
    let target = nadir.altitude + (peak.altitude - nadir.altitude) * asr_ratio;

    // Find descending crossing of this target after peak
    let after_peak: Vec<&AltitudeSample> = samples.iter()
        .filter(|s| s.seconds >= peak.seconds)
        .collect();

    for w in after_peak.windows(2) {
        if w[0].altitude >= target && w[1].altitude < target {
            let frac = (target - w[0].altitude) / (w[1].altitude - w[0].altitude);
            return w[0].seconds + frac * (w[1].seconds - w[0].seconds);
        }
    }

    // Fallback: 55% of afternoon arc
    let half_cycle = wrapped_duration(peak.seconds, nadir.seconds);
    (peak.seconds + half_cycle * 0.55) % 86400.0
}

// ─── Twilight computation (normalized wave) ─────────────────────

/// Map a twilight angle to a time using the normalized wave shape.
///
/// Instead of simple proportional mapping, this normalizes the full
/// altitude wave to [0, 1] and finds where the normalized target
/// falls on the actual curve. This preserves the sinusoidal shape.
fn wave_mapped_time(
    samples: &[AltitudeSample],
    peak: &AltitudeSample,
    nadir: &AltitudeSample,
    target_angle: f64,
    ascending: bool,
) -> f64 {
    // First try direct crossing (if the wave actually reaches this angle)
    if let Some(secs) = solar::find_crossing(samples, target_angle, ascending) {
        return secs;
    }

    // Normalize target within the wave
    let norm_target = solar::normalize_wave(target_angle, nadir.altitude, peak.altitude);

    // Target altitude on the actual wave
    let mapped_alt = nadir.altitude + norm_target * (peak.altitude - nadir.altitude);

    // Find crossing on the correct limb
    if ascending {
        // Search from nadir forward to peak
        let start = nadir.seconds;
        let candidates: Vec<&AltitudeSample> = if peak.seconds > nadir.seconds {
            samples.iter().filter(|s| s.seconds >= start && s.seconds <= peak.seconds).collect()
        } else {
            samples.iter().filter(|s| s.seconds >= start || s.seconds <= peak.seconds).collect()
        };
        for w in candidates.windows(2) {
            if w[0].altitude <= mapped_alt && w[1].altitude > mapped_alt {
                let frac = (mapped_alt - w[0].altitude) / (w[1].altitude - w[0].altitude);
                return w[0].seconds + frac * (w[1].seconds - w[0].seconds);
            }
        }
    } else {
        // Search from peak forward to nadir
        let candidates: Vec<&AltitudeSample> = if nadir.seconds > peak.seconds {
            samples.iter().filter(|s| s.seconds >= peak.seconds && s.seconds <= nadir.seconds).collect()
        } else {
            samples.iter().filter(|s| s.seconds >= peak.seconds || s.seconds <= nadir.seconds).collect()
        };
        for w in candidates.windows(2) {
            if w[0].altitude >= mapped_alt && w[1].altitude < mapped_alt {
                let frac = (mapped_alt - w[0].altitude) / (w[1].altitude - w[0].altitude);
                return w[0].seconds + frac * (w[1].seconds - w[0].seconds);
            }
        }
    }

    // Final fallback: proportional time
    let half = wrapped_duration(nadir.seconds, peak.seconds);
    if ascending {
        (nadir.seconds + half * norm_target) % 86400.0
    } else {
        (peak.seconds + half * (1.0 - norm_target)) % 86400.0
    }
}

// ─── Utility ────────────────────────────────────────────────────

fn wrapped_duration(from: f64, to: f64) -> f64 {
    if to > from { to - from } else { to + 86400.0 - from }
}

// ─── Schedule builders ──────────────────────────────────────────

pub fn compute_schedule(date: NaiveDate, lat: f64, lon: f64, strategy: GapStrategy) -> Schedule {
    let samples = solar::day_scan(date, lat, lon, 30);
    let peak = solar::find_peak(&samples);
    let nadir = solar::find_nadir(&samples);
    let state = classify_day(&samples);

    let solar_info = SolarInfo {
        max_altitude: peak.altitude,
        min_altitude: nadir.altitude,
        peak_utc: solar::seconds_to_hms(peak.seconds),
        nadir_utc: solar::seconds_to_hms(nadir.seconds),
    };

    let mut events = match state {
        DayState::Normal => build_normal(&samples, &peak, &nadir),
        DayState::MidnightSun => build_midnight_sun(&samples, &peak, &nadir),
        DayState::PolarNight => build_polar_night(&samples, &peak, &nadir),
    };

    if strategy == GapStrategy::Projected45 && state != DayState::Normal {
        apply_projection(&mut events, date, lat, lon);
    }

    Schedule { state, events, solar: solar_info }
}

/// Compute the adaptive reference latitude for projection (Aqrab al-Bilad).
///
/// Instead of a fixed 45°, the reference scales with the user's latitude:
/// - Tropical (<30°): use 45° (standard, projection rarely needed)
/// - Temperate (30-60°): use the user's own latitude (closest normal day)
/// - Polar (>60°): use lat - 15° (step back toward temperate zone)
pub fn compute_reference_lat(lat: f64) -> f64 {
    let abs_lat = lat.abs();
    let ref_abs = if abs_lat < 30.0 {
        45.0
    } else if abs_lat < 60.0 {
        abs_lat
    } else {
        abs_lat - 15.0
    };
    if lat >= 0.0 { ref_abs } else { -ref_abs }
}

/// Project sunrise/maghrib from an adaptive reference latitude (Aqrab al-Bilad).
///
/// For polar states where sunrise/sunset don't exist physically, we:
/// 1. Compute an adaptive reference latitude based on user position
/// 2. Scan the same date at that reference to get sunrise/sunset durations
/// 3. Apply those durations relative to the user's local solar noon
fn apply_projection(events: &mut Events, date: NaiveDate, lat: f64, lon: f64) {
    let ref_lat = compute_reference_lat(lat);

    // Scan the reference day
    let ref_samples = solar::day_scan(date, ref_lat, lon, 30);
    let ref_peak = solar::find_peak(&ref_samples);

    // Find reference sunrise and sunset
    let ref_sunrise = solar::find_crossing(&ref_samples, HORIZON_ANGLE, true);
    let ref_sunset = solar::find_crossing(&ref_samples, HORIZON_ANGLE, false);

    // Both must exist at 45° for projection to work
    let (ref_sunrise_secs, ref_sunset_secs) = match (ref_sunrise, ref_sunset) {
        (Some(sr), Some(ss)) => (sr, ss),
        _ => return, // 45° has no sunrise/sunset — extremely unlikely, bail out
    };

    // Compute durations relative to reference noon
    let ref_noon = ref_peak.seconds;
    let morning_duration = wrapped_duration(ref_sunrise_secs, ref_noon);
    let evening_duration = wrapped_duration(ref_noon, ref_sunset_secs);

    // Get the user's local solar noon
    let local_samples = solar::day_scan(date, lat, lon, 30);
    let local_peak = solar::find_peak(&local_samples);
    let local_noon = local_peak.seconds;

    let note = format!("Adaptive projection anchored to {:.1}° reference latitude", ref_lat);

    // Project sunrise: local_noon - morning_duration
    if events.sunrise.method == EventMethod::None {
        let projected_sunrise = ((local_noon - morning_duration) % 86400.0 + 86400.0) % 86400.0;
        events.sunrise = PrayerEvent::projected(projected_sunrise, &note);
    }

    // Project maghrib: local_noon + evening_duration
    if events.maghrib.method == EventMethod::None {
        let projected_maghrib = (local_noon + evening_duration) % 86400.0;
        events.maghrib = PrayerEvent::projected(projected_maghrib, &note);
    }
}

fn build_normal(
    samples: &[AltitudeSample],
    peak: &AltitudeSample,
    nadir: &AltitudeSample,
) -> Events {
    let sunrise_secs = solar::find_crossing(samples, HORIZON_ANGLE, true)
        .unwrap_or(peak.seconds - 6.0 * 3600.0);
    let sunset_secs = solar::find_crossing(samples, HORIZON_ANGLE, false)
        .unwrap_or(peak.seconds + 6.0 * 3600.0);

    let dhuhr_secs = peak.seconds;

    // Asr: geometric (standard shadow formula)
    let asr_alt = standard_asr_altitude(peak.altitude);
    let asr_secs = solar::find_crossing(samples, asr_alt, false)
        .unwrap_or_else(|| virtual_asr_seconds(samples, peak, nadir));

    // Fajr/Isha: direct crossing or wave-mapped
    let fajr_secs = wave_mapped_time(samples, peak, nadir, FAJR_ANGLE, true);
    let isha_secs = wave_mapped_time(samples, peak, nadir, ISHA_ANGLE, false);

    let fajr_method = if solar::find_crossing(samples, FAJR_ANGLE, true).is_some() {
        EventMethod::Standard
    } else {
        EventMethod::Virtual
    };
    let isha_method = if solar::find_crossing(samples, ISHA_ANGLE, false).is_some() {
        EventMethod::Standard
    } else {
        EventMethod::Virtual
    };

    let fajr_confidence = if fajr_method == EventMethod::Standard { 1.0 } else { 0.7 };
    let isha_confidence = if isha_method == EventMethod::Standard { 1.0 } else { 0.7 };

    Events {
        fajr: PrayerEvent { time: Some(solar::seconds_to_hms(fajr_secs)), method: fajr_method, confidence: fajr_confidence, note: None, next_day: false },
        sunrise: PrayerEvent::standard(sunrise_secs),
        dhuhr: PrayerEvent::standard(dhuhr_secs),
        asr: PrayerEvent::standard(asr_secs),
        maghrib: PrayerEvent::standard(sunset_secs),
        isha: PrayerEvent { time: Some(solar::seconds_to_hms(isha_secs)), method: isha_method, confidence: isha_confidence, note: None, next_day: false },
    }
}

fn build_midnight_sun(
    samples: &[AltitudeSample],
    peak: &AltitudeSample,
    nadir: &AltitudeSample,
) -> Events {
    // Sun never sets → sunrise and maghrib DO NOT EXIST physically
    let dhuhr_secs = peak.seconds;

    // Asr: the sun does reach Asr altitude (it's above horizon all day)
    let asr_alt = standard_asr_altitude(peak.altitude);
    let asr_secs = solar::find_crossing(samples, asr_alt, false)
        .unwrap_or_else(|| virtual_asr_seconds(samples, peak, nadir));
    let asr_method = if solar::find_crossing(samples, asr_alt, false).is_some() {
        EventMethod::Standard
    } else {
        EventMethod::Virtual
    };
    let asr_confidence = if asr_method == EventMethod::Standard { 1.0 } else { 0.7 };

    let fajr_secs = wave_mapped_time(samples, peak, nadir, FAJR_ANGLE, true);
    let isha_secs = wave_mapped_time(samples, peak, nadir, ISHA_ANGLE, false);

    Events {
        fajr: PrayerEvent::virtual_event(fajr_secs),
        sunrise: PrayerEvent::none(),   // Sun never set, so it never rises
        dhuhr: PrayerEvent::standard(dhuhr_secs),
        asr: PrayerEvent { time: Some(solar::seconds_to_hms(asr_secs)), method: asr_method, confidence: asr_confidence, note: None, next_day: false },
        maghrib: PrayerEvent::none(),   // Sun never sets
        isha: PrayerEvent::virtual_event(isha_secs),
    }
}

fn build_polar_night(
    samples: &[AltitudeSample],
    peak: &AltitudeSample,
    nadir: &AltitudeSample,
) -> Events {
    // Sun never rises → sunrise and maghrib DO NOT EXIST physically
    let dhuhr_secs = peak.seconds; // Virtual noon at peak altitude (below horizon)

    // Fajr/Isha first — these define the virtual day boundaries
    let fajr_secs = wave_mapped_time(samples, peak, nadir, FAJR_ANGLE, true);
    let isha_secs = wave_mapped_time(samples, peak, nadir, ISHA_ANGLE, false);

    // Virtual Asr: placed proportionally in the afternoon of the virtual day.
    // The "virtual afternoon" runs from dhuhr to isha. In a standard day,
    // Asr falls at roughly 55-60% of the afternoon. We use the same ratio.
    let afternoon = wrapped_duration(dhuhr_secs, isha_secs);
    let asr_secs = (dhuhr_secs + afternoon * 0.55) % 86400.0;

    Events {
        fajr: PrayerEvent::virtual_event(fajr_secs),
        sunrise: PrayerEvent::none(),
        dhuhr: PrayerEvent::virtual_event(dhuhr_secs),
        asr: PrayerEvent::virtual_event(asr_secs),
        maghrib: PrayerEvent::none(),
        isha: PrayerEvent::virtual_event(isha_secs),
    }
}

/// Return the day scan samples (for debug-wave mode).
pub fn day_scan_samples(date: NaiveDate, lat: f64, lon: f64) -> Vec<AltitudeSample> {
    solar::day_scan(date, lat, lon, 30)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_mecca_normal_schedule() {
        let date = NaiveDate::from_ymd_opt(2026, 2, 14).unwrap();
        let schedule = compute_schedule(date, 21.4225, 39.8262, GapStrategy::Strict);

        println!("=== Mecca Feb 14, 2026 ===");
        println!("{}", serde_json::to_string_pretty(&schedule).unwrap());

        assert_eq!(schedule.state, DayState::Normal);
        assert_eq!(schedule.events.sunrise.method, EventMethod::Standard);
        assert_eq!(schedule.events.maghrib.method, EventMethod::Standard);
        assert!(schedule.events.sunrise.time.is_some());
        assert!(schedule.events.maghrib.time.is_some());

        // Ordering
        let e = &schedule.events;
        assert!(e.fajr.time.as_ref().unwrap() < e.sunrise.time.as_ref().unwrap());
        assert!(e.sunrise.time.as_ref().unwrap() < e.dhuhr.time.as_ref().unwrap());
        assert!(e.dhuhr.time.as_ref().unwrap() < e.asr.time.as_ref().unwrap());
        assert!(e.asr.time.as_ref().unwrap() < e.maghrib.time.as_ref().unwrap());
        assert!(e.maghrib.time.as_ref().unwrap() < e.isha.time.as_ref().unwrap());
    }

    #[test]
    fn test_tromso_edge_case() {
        let date = NaiveDate::from_ymd_opt(2026, 2, 14).unwrap();
        let schedule = compute_schedule(date, 69.6492, 18.9553, GapStrategy::Strict);

        println!("=== Tromsø Feb 14, 2026 ===");
        println!("{}", serde_json::to_string_pretty(&schedule).unwrap());

        assert_eq!(schedule.state, DayState::Normal);
        assert!(schedule.solar.max_altitude > 0.0 && schedule.solar.max_altitude < 10.0);
    }

    #[test]
    fn test_svalbard_polar_night_truthful() {
        let date = NaiveDate::from_ymd_opt(2025, 12, 21).unwrap();
        let schedule = compute_schedule(date, 78.2232, 15.6267, GapStrategy::Strict);

        println!("=== Svalbard Dec 21, 2025 ===");
        println!("{}", serde_json::to_string_pretty(&schedule).unwrap());

        assert_eq!(schedule.state, DayState::PolarNight);
        assert!(schedule.solar.max_altitude < 0.0);

        // CRITICAL: sunrise and maghrib must be None
        assert_eq!(schedule.events.sunrise.method, EventMethod::None);
        assert!(schedule.events.sunrise.time.is_none(), "Polar night must NOT have sunrise");
        assert_eq!(schedule.events.maghrib.method, EventMethod::None);
        assert!(schedule.events.maghrib.time.is_none(), "Polar night must NOT have maghrib");

        // Virtual events must exist
        assert_eq!(schedule.events.dhuhr.method, EventMethod::Virtual);
        assert!(schedule.events.dhuhr.time.is_some());
        assert_eq!(schedule.events.asr.method, EventMethod::Virtual);
        assert!(schedule.events.asr.time.is_some());
        assert_eq!(schedule.events.fajr.method, EventMethod::Virtual);
        assert!(schedule.events.fajr.time.is_some());

        // Virtual ordering: fajr < dhuhr < asr < isha
        let e = &schedule.events;
        assert!(e.fajr.time.as_ref().unwrap() < e.dhuhr.time.as_ref().unwrap());
        assert!(e.dhuhr.time.as_ref().unwrap() < e.asr.time.as_ref().unwrap());
        assert!(e.asr.time.as_ref().unwrap() < e.isha.time.as_ref().unwrap());
    }

    #[test]
    fn test_midnight_sun_truthful() {
        let date = NaiveDate::from_ymd_opt(2026, 6, 21).unwrap();
        let schedule = compute_schedule(date, 69.6492, 18.9553, GapStrategy::Strict);

        println!("=== Tromsø Jun 21, 2026 (Midnight Sun) ===");
        println!("{}", serde_json::to_string_pretty(&schedule).unwrap());

        assert_eq!(schedule.state, DayState::MidnightSun);

        // CRITICAL: sunrise and maghrib must be None
        assert_eq!(schedule.events.sunrise.method, EventMethod::None);
        assert!(schedule.events.sunrise.time.is_none(), "Midnight sun must NOT have sunrise");
        assert_eq!(schedule.events.maghrib.method, EventMethod::None);
        assert!(schedule.events.maghrib.time.is_none(), "Midnight sun must NOT have maghrib");

        // Dhuhr is standard (peak is real)
        assert_eq!(schedule.events.dhuhr.method, EventMethod::Standard);
    }

    #[test]
    fn test_standard_asr_altitude() {
        let asr_alt = standard_asr_altitude(60.0);
        println!("Asr altitude for peak 60°: {:.4}°", asr_alt);
        assert!((asr_alt - 32.37).abs() < 0.5);

        let asr_alt_90 = standard_asr_altitude(90.0);
        println!("Asr altitude for peak 90°: {:.4}°", asr_alt_90);
        assert!((asr_alt_90 - 45.0).abs() < 0.1);
    }

    #[test]
    fn test_geometric_vs_standard_asr() {
        // Both formulas should give equivalent results for normal peaks
        for peak in [30.0, 45.0, 60.0, 75.0, 90.0] {
            let geo = geometric_asr_altitude(peak);
            let std = standard_asr_altitude(peak);
            println!("Peak {:.0}°: geometric={:.4}°, standard={:.4}°", peak, geo, std);
            // They use different formulations but should converge
            assert!((geo - std).abs() < 1.0,
                "Geometric and standard Asr diverge too much at peak {}°", peak);
        }
    }

    // ─── v6 Projection Tests ─────────────────────────────────────

    #[test]
    fn test_tromso_jun21_strict_no_maghrib() {
        let date = NaiveDate::from_ymd_opt(2026, 6, 21).unwrap();
        let schedule = compute_schedule(date, 69.6492, 18.9553, GapStrategy::Strict);
        assert_eq!(schedule.state, DayState::MidnightSun);
        assert!(schedule.events.maghrib.time.is_none());
        assert_eq!(schedule.events.maghrib.method, EventMethod::None);
    }

    #[test]
    fn test_tromso_jun21_projected45_has_maghrib() {
        let date = NaiveDate::from_ymd_opt(2026, 6, 21).unwrap();
        let schedule = compute_schedule(date, 69.6492, 18.9553, GapStrategy::Projected45);
        assert_eq!(schedule.state, DayState::MidnightSun);

        // Maghrib should now be filled via projection
        assert!(schedule.events.maghrib.time.is_some(), "Projected45 must fill maghrib");
        assert_eq!(schedule.events.maghrib.method, EventMethod::Projected);
        assert!(schedule.events.maghrib.note.is_some(), "Projected event must have note");

        // Sunrise should also be filled
        assert!(schedule.events.sunrise.time.is_some(), "Projected45 must fill sunrise");
        assert_eq!(schedule.events.sunrise.method, EventMethod::Projected);

        // Projected maghrib should be in a reasonable UTC range (afternoon/evening)
        let maghrib_secs = hms_to_seconds(schedule.events.maghrib.time.as_ref().unwrap());
        assert!(maghrib_secs > 14.0 * 3600.0 && maghrib_secs < 23.0 * 3600.0,
            "Projected maghrib should be between 14:00-23:00 UTC, got {}",
            schedule.events.maghrib.time.as_ref().unwrap());
    }

    #[test]
    fn test_svalbard_dec21_projected45_full_schedule() {
        let date = NaiveDate::from_ymd_opt(2025, 12, 21).unwrap();
        let schedule = compute_schedule(date, 78.2232, 15.6267, GapStrategy::Projected45);
        assert_eq!(schedule.state, DayState::PolarNight);

        // Sunrise and maghrib should be filled
        assert!(schedule.events.sunrise.time.is_some(), "Projected45 must fill sunrise in polar night");
        assert!(schedule.events.maghrib.time.is_some(), "Projected45 must fill maghrib in polar night");
        assert_eq!(schedule.events.sunrise.method, EventMethod::Projected);
        assert_eq!(schedule.events.maghrib.method, EventMethod::Projected);

        // Ordering: projected sunrise < noon < projected maghrib
        let sr = hms_to_seconds(schedule.events.sunrise.time.as_ref().unwrap());
        let noon = hms_to_seconds(schedule.events.dhuhr.time.as_ref().unwrap());
        let mg = hms_to_seconds(schedule.events.maghrib.time.as_ref().unwrap());
        assert!(sr < noon, "Projected sunrise ({}) must be before noon ({})", sr, noon);
        assert!(noon < mg, "Noon ({}) must be before projected maghrib ({})", noon, mg);
    }

    #[test]
    fn test_mecca_normal_unaffected_by_strategy() {
        let date = NaiveDate::from_ymd_opt(2026, 2, 14).unwrap();
        let strict = compute_schedule(date, 21.4225, 39.8262, GapStrategy::Strict);
        let projected = compute_schedule(date, 21.4225, 39.8262, GapStrategy::Projected45);

        // Normal days are identical regardless of strategy
        assert_eq!(strict.state, DayState::Normal);
        assert_eq!(projected.state, DayState::Normal);
        assert_eq!(strict.events.sunrise.time, projected.events.sunrise.time);
        assert_eq!(strict.events.maghrib.time, projected.events.maghrib.time);
        assert_eq!(strict.events.sunrise.method, projected.events.sunrise.method);
        assert_eq!(strict.events.maghrib.method, projected.events.maghrib.method);
    }

    #[test]
    fn test_projection_ordering_invariant() {
        // Projected sunrise must always be before projected maghrib
        let cases = vec![
            (69.6492, 18.9553, 2026, 6, 21),   // Tromsø, midnight sun
            (78.2232, 15.6267, 2025, 12, 21),   // Svalbard, polar night
        ];
        for (lat, lon, y, m, d) in cases {
            let date = NaiveDate::from_ymd_opt(y, m, d).unwrap();
            let schedule = compute_schedule(date, lat, lon, GapStrategy::Projected45);

            if let (Some(ref sr), Some(ref mg)) = (&schedule.events.sunrise.time, &schedule.events.maghrib.time) {
                let sr_secs = hms_to_seconds(sr);
                let mg_secs = hms_to_seconds(mg);
                assert!(sr_secs < mg_secs,
                    "Projected sunrise ({}) must be before maghrib ({}) at ({}, {})",
                    sr, mg, lat, lon);
            }
        }
    }

    // ─── v6.2 Production Upgrade Tests ──────────────────────────

    #[test]
    fn test_confidence_standard_events() {
        let date = NaiveDate::from_ymd_opt(2026, 2, 14).unwrap();
        let schedule = compute_schedule(date, 21.4225, 39.8262, GapStrategy::Strict);
        assert_eq!(schedule.events.sunrise.confidence, 1.0);
        assert_eq!(schedule.events.dhuhr.confidence, 1.0);
        assert_eq!(schedule.events.asr.confidence, 1.0);
        assert_eq!(schedule.events.maghrib.confidence, 1.0);
    }

    #[test]
    fn test_confidence_virtual_events() {
        let date = NaiveDate::from_ymd_opt(2025, 12, 21).unwrap();
        let schedule = compute_schedule(date, 78.2232, 15.6267, GapStrategy::Strict);
        assert_eq!(schedule.events.fajr.confidence, 0.7);
        assert_eq!(schedule.events.dhuhr.confidence, 0.7);
        assert_eq!(schedule.events.asr.confidence, 0.7);
        assert_eq!(schedule.events.isha.confidence, 0.7);
    }

    #[test]
    fn test_confidence_projected_events() {
        let date = NaiveDate::from_ymd_opt(2025, 12, 21).unwrap();
        let schedule = compute_schedule(date, 78.2232, 15.6267, GapStrategy::Projected45);
        assert_eq!(schedule.events.sunrise.confidence, 0.5);
        assert_eq!(schedule.events.maghrib.confidence, 0.5);
    }

    #[test]
    fn test_confidence_none_events() {
        let date = NaiveDate::from_ymd_opt(2025, 12, 21).unwrap();
        let schedule = compute_schedule(date, 78.2232, 15.6267, GapStrategy::Strict);
        assert_eq!(schedule.events.sunrise.confidence, 0.0);
        assert_eq!(schedule.events.maghrib.confidence, 0.0);
    }

    #[test]
    fn test_dynamic_ref_lat_tromso() {
        // Tromsø ~69.6° → ref = 69.6 - 15 = 54.6
        let ref_lat = compute_reference_lat(69.6492);
        assert!((ref_lat - 54.6).abs() < 0.1,
            "Tromsø ref_lat should be ~54.6, got {}", ref_lat);
        assert!(ref_lat != 45.0, "Tromsø must NOT use fixed 45°");
    }

    #[test]
    fn test_dynamic_ref_lat_svalbard() {
        // Svalbard ~78.2° → ref = 78.2 - 15 = 63.2
        let ref_lat = compute_reference_lat(78.2232);
        assert!((ref_lat - 63.2).abs() < 0.1,
            "Svalbard ref_lat should be ~63.2, got {}", ref_lat);
        assert!(ref_lat != 45.0, "Svalbard must NOT use fixed 45°");
    }

    #[test]
    fn test_dynamic_ref_lat_southern_hemisphere() {
        // Southern polar location: -70° → ref = -(70 - 15) = -55
        let ref_lat = compute_reference_lat(-70.0);
        assert!((ref_lat - (-55.0)).abs() < 0.1,
            "Southern 70° ref_lat should be -55, got {}", ref_lat);
        assert!(ref_lat < 0.0, "Southern hemisphere ref must be negative");
    }

    #[test]
    fn test_dynamic_ref_lat_tropical() {
        // Mecca ~21.4° → ref = 45 (tropical fallback)
        let ref_lat = compute_reference_lat(21.4225);
        assert_eq!(ref_lat, 45.0, "Tropical locations should use 45°");
    }

    #[test]
    fn test_dynamic_ref_lat_temperate() {
        // Stockholm ~59.3° → ref = 59.3 (temperate identity)
        let ref_lat = compute_reference_lat(59.3);
        assert!((ref_lat - 59.3).abs() < 0.1,
            "Temperate ref_lat should equal input, got {}", ref_lat);
    }

    #[test]
    fn test_projection_note_reflects_dynamic_lat() {
        let date = NaiveDate::from_ymd_opt(2026, 6, 21).unwrap();
        let schedule = compute_schedule(date, 69.6492, 18.9553, GapStrategy::Projected45);
        let note = schedule.events.maghrib.note.as_ref().unwrap();
        // Note should mention the dynamic reference lat (~54.6), not 45
        assert!(note.contains("54."), "Note should reflect dynamic ref lat, got: {}", note);
    }

    #[test]
    fn test_mecca_regression_unchanged_v62() {
        // Full regression: Mecca Feb 14 must be identical to v6 outputs
        let date = NaiveDate::from_ymd_opt(2026, 2, 14).unwrap();
        let schedule = compute_schedule(date, 21.4225, 39.8262, GapStrategy::Strict);

        assert_eq!(schedule.state, DayState::Normal);
        assert_eq!(schedule.events.sunrise.method, EventMethod::Standard);
        assert_eq!(schedule.events.maghrib.method, EventMethod::Standard);
        assert_eq!(schedule.events.sunrise.confidence, 1.0);
        assert_eq!(schedule.events.maghrib.confidence, 1.0);
        assert!(!schedule.events.sunrise.next_day);
        assert!(!schedule.events.maghrib.next_day);
        assert!(schedule.events.sunrise.note.is_none());
        assert!(schedule.events.maghrib.note.is_none());

        // Ordering must still hold
        let e = &schedule.events;
        assert!(e.fajr.time.as_ref().unwrap() < e.sunrise.time.as_ref().unwrap());
        assert!(e.sunrise.time.as_ref().unwrap() < e.dhuhr.time.as_ref().unwrap());
        assert!(e.dhuhr.time.as_ref().unwrap() < e.asr.time.as_ref().unwrap());
        assert!(e.asr.time.as_ref().unwrap() < e.maghrib.time.as_ref().unwrap());
        assert!(e.maghrib.time.as_ref().unwrap() < e.isha.time.as_ref().unwrap());
    }
}
