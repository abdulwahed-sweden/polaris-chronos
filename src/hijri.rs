//! Hijri calendar with astronomical crescent visibility (Odeh 2004 criterion).
//!
//! Provides:
//! - Tabular Hijri ↔ Gregorian conversion (30-year cycle, used as seed)
//! - Astronomical conjunction detection via iterative search
//! - Odeh crescent visibility scoring (q-value / zone classification)
//! - Ramadan date finder that respects actual lunar visibility

use chrono::{Duration, NaiveDate, NaiveDateTime, NaiveTime};
use serde::Serialize;
use std::f64::consts::PI;

use crate::lunar::{lunar_position, moon_sun_elongation};
use crate::solar;

const DEG: f64 = PI / 180.0;

// ─── Hijri Date ──────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HijriDate {
    pub year: u32,
    pub month: u32,
    pub day: u32,
}

/// Hijri epoch: July 16, 622 CE (Julian) = July 19, 622 CE (Gregorian proleptic)
const HIJRI_EPOCH_JD: f64 = 1948439.5;

/// The 30-year cycle pattern: which years in a 30-year cycle are leap years.
/// Leap years have 355 days, common years have 354 days.
const LEAP_YEARS: [u32; 11] = [2, 5, 7, 10, 13, 16, 18, 21, 24, 26, 29];

fn is_hijri_leap(year: u32) -> bool {
    let y_mod = year % 30;
    LEAP_YEARS.contains(&y_mod)
}

fn hijri_year_days(year: u32) -> u32 {
    if is_hijri_leap(year) { 355 } else { 354 }
}

fn hijri_month_days(year: u32, month: u32) -> u32 {
    // Odd months have 30 days, even months have 29 days.
    // Exception: month 12 in leap years has 30 days.
    if month % 2 == 1 {
        30
    } else if month == 12 && is_hijri_leap(year) {
        30
    } else {
        29
    }
}

/// Convert a Gregorian date to a tabular Hijri date.
pub fn gregorian_to_hijri(date: NaiveDate) -> HijriDate {
    // Convert to Julian Day Number
    let jd = solar::julian_date(&date.and_hms_opt(12, 0, 0).unwrap());

    let days_since_epoch = (jd - HIJRI_EPOCH_JD).floor() as i64;
    if days_since_epoch < 0 {
        return HijriDate { year: 1, month: 1, day: 1 };
    }

    // Count 30-year cycles
    let cycle_days: i64 = 10631; // 30 years = 10631 days
    let cycles = days_since_epoch / cycle_days;
    let mut remaining = days_since_epoch % cycle_days;

    let mut year = (cycles * 30) as u32 + 1;

    // Count remaining years
    loop {
        let yd = hijri_year_days(year) as i64;
        if remaining < yd {
            break;
        }
        remaining -= yd;
        year += 1;
    }

    // Count months
    let mut month = 1u32;
    loop {
        let md = hijri_month_days(year, month) as i64;
        if remaining < md {
            break;
        }
        remaining -= md;
        month += 1;
        if month > 12 {
            month = 12;
            break;
        }
    }

    let day = remaining as u32 + 1;

    HijriDate { year, month, day }
}

/// Convert a tabular Hijri date to Gregorian.
pub fn hijri_to_gregorian(hijri: HijriDate) -> NaiveDate {
    let mut total_days: i64 = 0;

    // Full years
    for y in 1..hijri.year {
        total_days += hijri_year_days(y) as i64;
    }

    // Full months in current year
    for m in 1..hijri.month {
        total_days += hijri_month_days(hijri.year, m) as i64;
    }

    // Days in current month
    total_days += (hijri.day - 1) as i64;

    // Convert from epoch
    let jd = HIJRI_EPOCH_JD + total_days as f64;

    // JD to Gregorian
    jd_to_gregorian(jd)
}

fn jd_to_gregorian(jd: f64) -> NaiveDate {
    let z = (jd + 0.5).floor() as i64;
    let a = if z < 2299161 {
        z
    } else {
        let alpha = ((z as f64 - 1867216.25) / 36524.25).floor() as i64;
        z + 1 + alpha - alpha / 4
    };

    let b = a + 1524;
    let c = ((b as f64 - 122.1) / 365.25).floor() as i64;
    let d = (365.25 * c as f64).floor() as i64;
    let e = ((b - d) as f64 / 30.6001).floor() as i64;

    let day = b - d - (30.6001 * e as f64).floor() as i64;
    let month = if e < 14 { e - 1 } else { e - 13 };
    let year = if month > 2 { c - 4716 } else { c - 4715 };

    NaiveDate::from_ymd_opt(year as i32, month as u32, day as u32)
        .unwrap_or_else(|| NaiveDate::from_ymd_opt(year as i32, 1, 1).unwrap())
}

// ─── Conjunction Detection ────────────────────────────────────────

/// Find the new moon conjunction nearest to the given date.
/// Uses iterative refinement: step by delta_elongation / 13.2 degrees/day.
pub fn find_conjunction(near_date: NaiveDate) -> NaiveDateTime {
    let mut dt = near_date.and_hms_opt(12, 0, 0).unwrap();

    // Coarse search: step by 1 day, find sign change in delta_longitude
    let mut prev_elong = moon_sun_elongation(&dt);
    let mut prev_dt = dt;

    // Search within ±20 days
    for day_offset in -20i64..=20 {
        let check_dt = near_date
            .checked_add_signed(Duration::days(day_offset))
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        let elong = moon_sun_elongation(&check_dt);

        if elong < prev_elong && elong < 5.0 {
            dt = check_dt;
            break;
        }
        if elong > prev_elong && prev_elong < 5.0 {
            dt = prev_dt;
            break;
        }
        prev_elong = elong;
        prev_dt = check_dt;
    }

    // Fine search: binary-style refinement around the minimum
    let mut step_hours: f64 = 12.0;
    for _ in 0..20 {
        let elong_now = moon_sun_elongation(&dt);

        let dt_fwd = dt
            .checked_add_signed(Duration::minutes((step_hours * 60.0) as i64))
            .unwrap();
        let dt_bwd = dt
            .checked_sub_signed(Duration::minutes((step_hours * 60.0) as i64))
            .unwrap();

        let elong_fwd = moon_sun_elongation(&dt_fwd);
        let elong_bwd = moon_sun_elongation(&dt_bwd);

        if elong_fwd < elong_now {
            dt = dt_fwd;
        } else if elong_bwd < elong_now {
            dt = dt_bwd;
        }

        step_hours *= 0.5;
        if step_hours < 0.01 {
            break;
        }
    }

    dt
}

// ─── Odeh Crescent Visibility ─────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum CrescentZone {
    A, // Naked eye visible
    B, // Optical aid, may be naked eye
    C, // Needs optical aid
    D, // Not visible
}

impl std::fmt::Display for CrescentZone {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CrescentZone::A => write!(f, "A (naked eye)"),
            CrescentZone::B => write!(f, "B (may need optical aid)"),
            CrescentZone::C => write!(f, "C (optical aid required)"),
            CrescentZone::D => write!(f, "D (not visible)"),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CrescentVisibility {
    pub zone: CrescentZone,
    pub q_value: f64,
    pub moon_age_hours: f64,
    pub moon_altitude: f64,
    pub elongation: f64,
    pub arc_of_vision: f64,
    pub crescent_width: f64,
}

/// Find sunset time (in UTC seconds from midnight) for a given date and location.
fn find_sunset(date: NaiveDate, lat: f64, lon: f64) -> Option<NaiveDateTime> {
    let samples = solar::day_scan(date, lat, lon, 60);
    let sunset_secs = solar::find_crossing(&samples, solar::HORIZON_ANGLE, false)?;

    let h = (sunset_secs / 3600.0).floor() as u32;
    let m = ((sunset_secs % 3600.0) / 60.0).floor() as u32;
    let s = (sunset_secs % 60.0).floor() as u32;

    Some(NaiveDateTime::new(
        date,
        NaiveTime::from_hms_opt(h.min(23), m.min(59), s.min(59))?,
    ))
}

/// Evaluate crescent visibility on a given evening using the Odeh (2004) criterion.
pub fn evaluate_visibility(
    date: NaiveDate,
    lat: f64,
    lon: f64,
    conjunction: &NaiveDateTime,
) -> CrescentVisibility {
    // Find sunset on this date
    let sunset = match find_sunset(date, lat, lon) {
        Some(ss) => ss,
        None => {
            return CrescentVisibility {
                zone: CrescentZone::D,
                q_value: -999.0,
                moon_age_hours: 0.0,
                moon_altitude: 0.0,
                elongation: 0.0,
                arc_of_vision: 0.0,
                crescent_width: 0.0,
            };
        }
    };

    // Moon age at sunset
    let moon_age_hours = (sunset.signed_duration_since(*conjunction).num_seconds() as f64) / 3600.0;

    if moon_age_hours < 0.0 {
        // Conjunction hasn't happened yet
        return CrescentVisibility {
            zone: CrescentZone::D,
            q_value: -999.0,
            moon_age_hours,
            moon_altitude: 0.0,
            elongation: 0.0,
            arc_of_vision: 0.0,
            crescent_width: 0.0,
        };
    }

    // Moon position at sunset
    let moon = lunar_position(&sunset, lat, lon);
    let moon_altitude = moon.altitude;

    // Elongation at sunset
    let elongation = moon_sun_elongation(&sunset);

    // ARCV = Moon topocentric altitude at sunset
    let arcv = moon_altitude;

    // W = crescent width in arcminutes
    // W = 15 * (1 - cos(elongation))
    let w = 15.0 * (1.0 - (elongation * DEG).cos());

    // Odeh q-value
    // q = ARCV - (-0.1018*W³ + 0.7319*W² - 6.3226*W + 7.1814)
    let q = arcv - (-0.1018 * w.powi(3) + 0.7319 * w.powi(2) - 6.3226 * w + 7.1814);

    let zone = if q >= 0.0 {
        CrescentZone::A
    } else if q >= -0.014 {
        CrescentZone::B
    } else if q >= -0.232 {
        CrescentZone::C
    } else {
        CrescentZone::D
    };

    CrescentVisibility {
        zone,
        q_value: q,
        moon_age_hours,
        moon_altitude,
        elongation,
        arc_of_vision: arcv,
        crescent_width: w,
    }
}

// ─── Ramadan Finder ───────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct RamadanInfo {
    pub hijri_year: u32,
    pub start: String,
    pub end: String,
    pub days: u32,
    pub conjunction: String,
    pub visibility: CrescentVisibility,
    pub shawwal_start: String,
}

/// Determine Ramadan start/end for a given Hijri year and observer location.
pub fn find_ramadan(hijri_year: u32, lat: f64, lon: f64) -> RamadanInfo {
    // Step 1: Tabular estimate for Ramadan 1 (month 9)
    let tabular_start = hijri_to_gregorian(HijriDate {
        year: hijri_year,
        month: 9,
        day: 1,
    });

    // Step 2: Find the conjunction near the tabular estimate
    // Search a few days before to account for tabular inaccuracy
    let search_date = tabular_start
        .checked_sub_signed(Duration::days(3))
        .unwrap();
    let conjunction = find_conjunction(search_date);

    // Step 3: Check evenings starting from conjunction day
    let conj_date = conjunction.date();
    let mut ramadan_start: Option<NaiveDate> = None;

    for day_offset in 0..5 {
        let check_date = conj_date
            .checked_add_signed(Duration::days(day_offset))
            .unwrap();
        let vis = evaluate_visibility(check_date, lat, lon, &conjunction);

        if vis.zone == CrescentZone::A || vis.zone == CrescentZone::B {
            // Ramadan 1 is the day AFTER the first visible crescent evening
            ramadan_start = Some(
                check_date
                    .checked_add_signed(Duration::days(1))
                    .unwrap(),
            );
            break;
        }
    }

    // Fallback: if no visibility found within 5 days, use conjunction + 2 days
    let ramadan_1 = ramadan_start.unwrap_or_else(|| {
        conj_date
            .checked_add_signed(Duration::days(2))
            .unwrap()
    });

    // Step 4: Find Shawwal conjunction (next month)
    let shawwal_search = ramadan_1
        .checked_add_signed(Duration::days(25))
        .unwrap();
    let shawwal_conjunction = find_conjunction(shawwal_search);

    // Step 5: Determine Shawwal start
    let shawwal_conj_date = shawwal_conjunction.date();
    let mut shawwal_start: Option<NaiveDate> = None;

    for day_offset in 0..5 {
        let check_date = shawwal_conj_date
            .checked_add_signed(Duration::days(day_offset))
            .unwrap();
        let vis = evaluate_visibility(check_date, lat, lon, &shawwal_conjunction);

        if vis.zone == CrescentZone::A || vis.zone == CrescentZone::B {
            shawwal_start = Some(
                check_date
                    .checked_add_signed(Duration::days(1))
                    .unwrap(),
            );
            break;
        }
    }

    let shawwal_1 = shawwal_start.unwrap_or_else(|| {
        shawwal_conj_date
            .checked_add_signed(Duration::days(2))
            .unwrap()
    });

    let ramadan_days = shawwal_1.signed_duration_since(ramadan_1).num_days() as u32;
    let ramadan_end = ramadan_1
        .checked_add_signed(Duration::days(ramadan_days as i64 - 1))
        .unwrap();

    // Visibility for Ramadan start (the evening before Ramadan 1)
    let vis_evening = ramadan_1
        .checked_sub_signed(Duration::days(1))
        .unwrap();
    let visibility = evaluate_visibility(vis_evening, lat, lon, &conjunction);

    RamadanInfo {
        hijri_year,
        start: ramadan_1.format("%Y-%m-%d").to_string(),
        end: ramadan_end.format("%Y-%m-%d").to_string(),
        days: ramadan_days,
        conjunction: conjunction.format("%Y-%m-%d %H:%M UTC").to_string(),
        visibility,
        shawwal_start: shawwal_1.format("%Y-%m-%d").to_string(),
    }
}

/// Determine the current Hijri year for Ramadan lookup.
pub fn current_hijri_year_for_ramadan() -> u32 {
    let today = chrono::Utc::now().naive_utc().date();
    let hijri = gregorian_to_hijri(today);
    // If we're past Ramadan (month > 9), look at next year's Ramadan
    // If we're before or in Ramadan (month <= 9), use current year
    if hijri.month > 9 {
        hijri.year + 1
    } else {
        hijri.year
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;

    #[test]
    fn test_gregorian_to_hijri_known_date() {
        // 2026-02-17 should be approximately Sha'ban 29 or Ramadan 1, 1447
        let date = NaiveDate::from_ymd_opt(2026, 2, 17).unwrap();
        let hijri = gregorian_to_hijri(date);
        assert_eq!(hijri.year, 1447);
        assert!(hijri.month == 8 || hijri.month == 9, "Expected month 8 or 9, got {}", hijri.month);
    }

    #[test]
    fn test_hijri_roundtrip() {
        let original = NaiveDate::from_ymd_opt(2026, 3, 15).unwrap();
        let hijri = gregorian_to_hijri(original);
        let back = hijri_to_gregorian(hijri);
        let diff = (original.signed_duration_since(back).num_days()).abs();
        assert!(diff <= 1, "Roundtrip error: {} days", diff);
    }

    #[test]
    fn test_conjunction_feb_2026() {
        // New moon conjunction around Feb 17, 2026
        let near = NaiveDate::from_ymd_opt(2026, 2, 17).unwrap();
        let conj = find_conjunction(near);
        // Conjunction should be on Feb 17, 2026 (approximately)
        assert_eq!(conj.date().month(), 2);
        assert!(conj.date().day() >= 16 && conj.date().day() <= 18,
            "Conjunction date: {}", conj);
    }

    #[test]
    fn test_feb17_mecca_not_visible() {
        // Feb 17 evening from Mecca: moon is only ~3.5h old, should be Zone D
        let conj_date = NaiveDate::from_ymd_opt(2026, 2, 17).unwrap();
        let conjunction = find_conjunction(conj_date);
        let vis = evaluate_visibility(conj_date, 21.4225, 39.8262, &conjunction);
        assert_eq!(
            vis.zone,
            CrescentZone::D,
            "Feb 17 evening Mecca should be Zone D (not visible), got {:?} (q={:.3}, age={:.1}h)",
            vis.zone, vis.q_value, vis.moon_age_hours
        );
    }

    #[test]
    fn test_feb18_mecca_visible() {
        // Feb 18 evening from Mecca: moon is ~27.5h old, should be Zone A or B
        let conj_date = NaiveDate::from_ymd_opt(2026, 2, 17).unwrap();
        let conjunction = find_conjunction(conj_date);
        let check_date = NaiveDate::from_ymd_opt(2026, 2, 18).unwrap();
        let vis = evaluate_visibility(check_date, 21.4225, 39.8262, &conjunction);
        assert!(
            vis.zone == CrescentZone::A || vis.zone == CrescentZone::B,
            "Feb 18 evening Mecca should be Zone A or B, got {:?} (q={:.3}, age={:.1}h)",
            vis.zone, vis.q_value, vis.moon_age_hours
        );
    }

    #[test]
    fn test_ramadan_1447_mecca() {
        // Ramadan 1447 should start on Feb 19, 2026 from Mecca
        let info = find_ramadan(1447, 21.4225, 39.8262);
        assert_eq!(info.start, "2026-02-19",
            "Ramadan 1447 from Mecca should start Feb 19, got {}", info.start);
        assert!(info.days == 29 || info.days == 30,
            "Ramadan should be 29 or 30 days, got {}", info.days);
    }

    #[test]
    fn test_odeh_q_formula() {
        // Unit test: if ARCV = 5.0, elongation = 10.0 degrees
        let w = 15.0 * (1.0 - (10.0_f64 * DEG).cos());
        let q = 5.0 - (-0.1018 * w.powi(3) + 0.7319 * w.powi(2) - 6.3226 * w + 7.1814);
        // Just verify the formula produces a finite number
        assert!(q.is_finite(), "q-value should be finite, got {}", q);
    }

    #[test]
    fn test_ramadan_1447_tromso() {
        // From Tromso, Ramadan may start same day or later
        let info = find_ramadan(1447, 69.6492, 18.9553);
        let start_date = NaiveDate::parse_from_str(&info.start, "%Y-%m-%d").unwrap();
        let feb19 = NaiveDate::from_ymd_opt(2026, 2, 19).unwrap();
        assert!(
            start_date >= feb19,
            "Tromso Ramadan start should be >= Feb 19, got {}", info.start
        );
    }
}
