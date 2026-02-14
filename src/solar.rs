//! Solar position calculator based on simplified SPA (Solar Position Algorithm).
//!
//! Computes altitude and azimuth for any instant, latitude, and longitude.
//! Accuracy: ~0.01° for dates within ±50 years of J2000.

use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use std::f64::consts::PI;

const DEG: f64 = PI / 180.0;
const ATMOSPHERIC_REFRACTION: f64 = 0.833;

/// Solar position at a specific instant.
#[derive(Debug, Clone, Copy)]
pub struct SolarPosition {
    pub altitude: f64,
    pub azimuth: f64,
    pub declination: f64,
    pub equation_of_time: f64,
}

/// A timestamped altitude sample from a day scan.
#[derive(Debug, Clone, Copy)]
pub struct AltitudeSample {
    pub seconds: f64,
    pub altitude: f64,
}

/// Convert a NaiveDateTime (assumed UTC) to Julian Date.
pub fn julian_date(dt: &NaiveDateTime) -> f64 {
    let y = dt.year() as f64;
    let m = dt.month() as f64;
    let d = dt.day() as f64;
    let h = dt.hour() as f64 + dt.minute() as f64 / 60.0 + dt.second() as f64 / 3600.0;

    let (y2, m2) = if m <= 2.0 {
        (y - 1.0, m + 12.0)
    } else {
        (y, m)
    };

    let a = (y2 / 100.0_f64).floor();
    let b = 2.0 - a + (a / 4.0_f64).floor();

    (365.25_f64 * (y2 + 4716.0)).floor()
        + (30.6001_f64 * (m2 + 1.0)).floor()
        + d
        + h / 24.0
        + b
        - 1524.5
}

fn julian_century(jd: f64) -> f64 {
    (jd - 2451545.0) / 36525.0
}

fn normalize_degrees(deg: f64) -> f64 {
    let mut d = deg % 360.0;
    if d < 0.0 {
        d += 360.0;
    }
    d
}

fn sun_mean_longitude(t: f64) -> f64 {
    normalize_degrees(280.46646 + t * (36000.76983 + t * 0.0003032))
}

fn sun_mean_anomaly(t: f64) -> f64 {
    normalize_degrees(357.52911 + t * (35999.05029 - t * 0.0001537))
}

fn earth_eccentricity(t: f64) -> f64 {
    0.016708634 - t * (0.000042037 + t * 0.0000001267)
}

fn sun_equation_of_center(t: f64) -> f64 {
    let m = sun_mean_anomaly(t) * DEG;
    m.sin() * (1.914602 - t * (0.004817 + t * 0.000014))
        + (2.0 * m).sin() * (0.019993 - t * 0.000101)
        + (3.0 * m).sin() * 0.000289
}

fn sun_true_longitude(t: f64) -> f64 {
    sun_mean_longitude(t) + sun_equation_of_center(t)
}

fn sun_apparent_longitude(t: f64) -> f64 {
    let omega = 125.04 - 1934.136 * t;
    sun_true_longitude(t) - 0.00569 - 0.00478 * (omega * DEG).sin()
}

fn mean_obliquity(t: f64) -> f64 {
    23.0 + (26.0 + (21.448 - t * (46.815 + t * (0.00059 - t * 0.001813))) / 60.0) / 60.0
}

fn obliquity_corrected(t: f64) -> f64 {
    let omega = 125.04 - 1934.136 * t;
    mean_obliquity(t) + 0.00256 * (omega * DEG).cos()
}

fn solar_declination(t: f64) -> f64 {
    let e = obliquity_corrected(t) * DEG;
    let lambda = sun_apparent_longitude(t) * DEG;
    (e.sin() * lambda.sin()).asin() / DEG
}

fn equation_of_time(t: f64) -> f64 {
    let e = obliquity_corrected(t) * DEG;
    let l0 = sun_mean_longitude(t) * DEG;
    let ecc = earth_eccentricity(t);
    let m = sun_mean_anomaly(t) * DEG;

    let y = (e / 2.0).tan().powi(2);

    let eq = y * (2.0 * l0).sin() - 2.0 * ecc * m.sin()
        + 4.0 * ecc * y * m.sin() * (2.0 * l0).cos()
        - 0.5 * y * y * (4.0 * l0).sin()
        - 1.25 * ecc * ecc * (2.0 * m).sin();

    4.0 * eq / DEG
}

/// Compute the solar position for a given UTC datetime, latitude, and longitude.
pub fn solar_position(dt: &NaiveDateTime, lat: f64, lon: f64) -> SolarPosition {
    let jd = julian_date(dt);
    let t = julian_century(jd);

    let decl = solar_declination(t);
    let eqt = equation_of_time(t);

    let hour = dt.hour() as f64 + dt.minute() as f64 / 60.0 + dt.second() as f64 / 3600.0;
    let solar_time = hour * 60.0 + eqt + 4.0 * lon;
    let hour_angle = solar_time / 4.0 - 180.0;

    let lat_r = lat * DEG;
    let decl_r = decl * DEG;
    let ha_r = hour_angle * DEG;

    let sin_alt = lat_r.sin() * decl_r.sin() + lat_r.cos() * decl_r.cos() * ha_r.cos();
    let altitude = sin_alt.asin() / DEG;

    let zenith = sin_alt.asin();
    let mut azimuth = if lat_r.cos().abs() > 1e-10 {
        let cos_az = (decl_r.sin() - zenith.sin() * lat_r.sin()) / (zenith.cos() * lat_r.cos());
        let az = cos_az.clamp(-1.0, 1.0).acos() / DEG;
        if hour_angle > 0.0 { 360.0 - az } else { az }
    } else {
        if decl > 0.0 { 180.0 } else { 0.0 }
    };
    azimuth = normalize_degrees(azimuth);

    SolarPosition { altitude, azimuth, declination: decl, equation_of_time: eqt }
}

/// Scan the full 24-hour solar altitude curve.
pub fn day_scan(date: NaiveDate, lat: f64, lon: f64, resolution_seconds: u32) -> Vec<AltitudeSample> {
    let mut samples = Vec::new();
    let mut sec = 0u32;
    while sec < 86400 {
        let h = sec / 3600;
        let m = (sec % 3600) / 60;
        let s = sec % 60;
        if let Some(time) = NaiveTime::from_hms_opt(h, m, s) {
            let dt = NaiveDateTime::new(date, time);
            let pos = solar_position(&dt, lat, lon);
            samples.push(AltitudeSample { seconds: sec as f64, altitude: pos.altitude });
        }
        sec += resolution_seconds;
    }
    samples
}

pub fn find_peak(samples: &[AltitudeSample]) -> AltitudeSample {
    *samples.iter().max_by(|a, b| a.altitude.partial_cmp(&b.altitude).unwrap()).unwrap()
}

pub fn find_nadir(samples: &[AltitudeSample]) -> AltitudeSample {
    *samples.iter().min_by(|a, b| a.altitude.partial_cmp(&b.altitude).unwrap()).unwrap()
}

/// Find the first crossing of a target altitude (ascending or descending).
/// Returns interpolated seconds from midnight, or None if no crossing occurs.
pub fn find_crossing(samples: &[AltitudeSample], target: f64, ascending: bool) -> Option<f64> {
    for window in samples.windows(2) {
        let (a, b) = (window[0], window[1]);
        let crosses = if ascending {
            a.altitude <= target && b.altitude > target
        } else {
            a.altitude >= target && b.altitude < target
        };
        if crosses {
            let frac = (target - a.altitude) / (b.altitude - a.altitude);
            return Some(a.seconds + frac * (b.seconds - a.seconds));
        }
    }
    None
}

/// Convert seconds from midnight to HH:MM:SS string.
pub fn seconds_to_hms(secs: f64) -> String {
    let total = secs.round() as i64;
    let total = ((total % 86400) + 86400) % 86400; // handle negatives
    let h = total / 3600;
    let m = (total % 3600) / 60;
    let s = total % 60;
    format!("{:02}:{:02}:{:02}", h, m, s)
}

/// Normalize the altitude wave to [0, 1] where 0 = nadir, 1 = peak.
pub fn normalize_wave(altitude: f64, min_alt: f64, max_alt: f64) -> f64 {
    let amplitude = max_alt - min_alt;
    if amplitude.abs() < 1e-10 {
        return 0.5;
    }
    ((altitude - min_alt) / amplitude).clamp(0.0, 1.0)
}

/// Refraction-adjusted horizon angle.
pub const HORIZON_ANGLE: f64 = -ATMOSPHERIC_REFRACTION;

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_cairo_solar_noon_equinox() {
        let date = NaiveDate::from_ymd_opt(2024, 3, 20).unwrap();
        let samples = day_scan(date, 30.0444, 31.2357, 60);
        let peak = find_peak(&samples);
        println!("Cairo equinox peak: {:.4}° at {}", peak.altitude, seconds_to_hms(peak.seconds));
        assert!((peak.altitude - 60.0).abs() < 1.5);
    }

    #[test]
    fn test_cairo_summer_solstice() {
        let date = NaiveDate::from_ymd_opt(2024, 6, 21).unwrap();
        let samples = day_scan(date, 30.0444, 31.2357, 60);
        let peak = find_peak(&samples);
        assert!(peak.altitude > 80.0);
    }

    #[test]
    fn test_cairo_sunrise_sunset() {
        let date = NaiveDate::from_ymd_opt(2024, 3, 20).unwrap();
        let samples = day_scan(date, 30.0444, 31.2357, 60);
        let sr = find_crossing(&samples, HORIZON_ANGLE, true).unwrap();
        let ss = find_crossing(&samples, HORIZON_ANGLE, false).unwrap();
        assert!(sr > 3.5 * 3600.0 && sr < 5.0 * 3600.0);
        assert!(ss > 15.5 * 3600.0 && ss < 17.0 * 3600.0);
    }

    #[test]
    fn test_mecca_feb14() {
        let date = NaiveDate::from_ymd_opt(2026, 2, 14).unwrap();
        let samples = day_scan(date, 21.4225, 39.8262, 60);
        let peak = find_peak(&samples);
        assert!(peak.altitude > 50.0 && peak.altitude < 65.0);
        assert!(find_crossing(&samples, HORIZON_ANGLE, true).is_some());
        assert!(find_crossing(&samples, HORIZON_ANGLE, false).is_some());
    }

    #[test]
    fn test_tromso_feb14() {
        let date = NaiveDate::from_ymd_opt(2026, 2, 14).unwrap();
        let samples = day_scan(date, 69.6492, 18.9553, 60);
        let peak = find_peak(&samples);
        assert!(peak.altitude > 0.0 && peak.altitude < 10.0);
    }

    #[test]
    fn test_svalbard_dec21() {
        let date = NaiveDate::from_ymd_opt(2025, 12, 21).unwrap();
        let samples = day_scan(date, 78.2232, 15.6267, 60);
        let peak = find_peak(&samples);
        assert!(peak.altitude < 0.0);
    }

    #[test]
    fn test_normalize_wave() {
        assert!((normalize_wave(-5.0, -10.0, 10.0) - 0.25).abs() < 1e-10);
        assert!((normalize_wave(10.0, -10.0, 10.0) - 1.0).abs() < 1e-10);
        assert!((normalize_wave(-10.0, -10.0, 10.0) - 0.0).abs() < 1e-10);
    }
}
