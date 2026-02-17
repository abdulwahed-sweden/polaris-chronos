//! Lunar position calculator based on Jean Meeus "Astronomical Algorithms" Ch. 47.
//!
//! Uses the top periodic terms from Tables 47.A and 47.B for ~0.3° accuracy,
//! sufficient for crescent visibility scoring.

use chrono::NaiveDateTime;
use std::f64::consts::PI;

use crate::solar::{julian_date, julian_century, normalize_degrees, obliquity_corrected, sun_ecliptic_longitude};

const DEG: f64 = PI / 180.0;

/// Lunar position at a specific instant.
#[derive(Debug, Clone, Copy)]
pub struct LunarPosition {
    pub longitude: f64,
    pub latitude: f64,
    pub distance_km: f64,
    pub right_ascension: f64,
    pub declination: f64,
    pub altitude: f64,
    pub azimuth: f64,
}

// Periodic terms for longitude and distance (Table 47.A)
// Each entry: (D, M, Mp, F, coeff_l, coeff_r)
// coeff_l in units of 0.000001 degrees, coeff_r in units of 0.001 km
const TERMS_LR: [(f64, f64, f64, f64, f64, f64); 20] = [
    (0.0, 0.0, 1.0, 0.0, 6288774.0, -20905355.0),
    (2.0, 0.0, -1.0, 0.0, 1274027.0, -3699111.0),
    (2.0, 0.0, 0.0, 0.0, 658314.0, -2955968.0),
    (0.0, 0.0, 2.0, 0.0, 213618.0, -569925.0),
    (0.0, 1.0, 0.0, 0.0, -185116.0, 48888.0),
    (0.0, 0.0, 0.0, 2.0, -114332.0, -3149.0),
    (2.0, 0.0, -2.0, 0.0, 58793.0, 246158.0),
    (2.0, -1.0, -1.0, 0.0, 57066.0, -152138.0),
    (2.0, 0.0, 1.0, 0.0, 53322.0, -170733.0),
    (2.0, -1.0, 0.0, 0.0, 45758.0, -204586.0),
    (0.0, 1.0, -1.0, 0.0, -40923.0, -129620.0),
    (1.0, 0.0, 0.0, 0.0, -34720.0, 108743.0),
    (0.0, 1.0, 1.0, 0.0, -30383.0, 104755.0),
    (2.0, 0.0, 0.0, -2.0, 15327.0, 10321.0),
    (0.0, 0.0, 1.0, 2.0, -12528.0, 0.0),
    (0.0, 0.0, 1.0, -2.0, 10980.0, 79661.0),
    (4.0, 0.0, -1.0, 0.0, 10675.0, -34782.0),
    (0.0, 0.0, 3.0, 0.0, 10034.0, -23210.0),
    (4.0, 0.0, -2.0, 0.0, 8548.0, -21636.0),
    (2.0, 1.0, -1.0, 0.0, -7888.0, 24208.0),
];

// Periodic terms for latitude (Table 47.B)
// Each entry: (D, M, Mp, F, coeff_b)
const TERMS_B: [(f64, f64, f64, f64, f64); 20] = [
    (0.0, 0.0, 0.0, 1.0, 5128122.0),
    (0.0, 0.0, 1.0, 1.0, 280602.0),
    (0.0, 0.0, 1.0, -1.0, 277693.0),
    (2.0, 0.0, 0.0, -1.0, 173237.0),
    (2.0, 0.0, -1.0, 1.0, 55413.0),
    (2.0, 0.0, -1.0, -1.0, 46271.0),
    (2.0, 0.0, 0.0, 1.0, 32573.0),
    (0.0, 0.0, 2.0, 1.0, 17198.0),
    (2.0, 0.0, 1.0, -1.0, 9266.0),
    (0.0, 0.0, 2.0, -1.0, 8822.0),
    (2.0, -1.0, 0.0, -1.0, 8216.0),
    (2.0, 0.0, -2.0, -1.0, 4324.0),
    (2.0, 0.0, 1.0, 1.0, 4200.0),
    (2.0, 1.0, 0.0, -1.0, -3359.0),
    (2.0, -1.0, -1.0, 1.0, 2463.0),
    (2.0, -1.0, 0.0, 1.0, 2211.0),
    (2.0, -1.0, -1.0, -1.0, 2065.0),
    (0.0, 1.0, -1.0, -1.0, -1870.0),
    (4.0, 0.0, -1.0, -1.0, 1828.0),
    (0.0, 1.0, 0.0, 1.0, -1794.0),
];

/// Moon mean longitude (L'), degrees
fn moon_mean_longitude(t: f64) -> f64 {
    normalize_degrees(
        218.3164477 + 481267.88123421 * t
            - 0.0015786 * t * t
            + t * t * t / 538841.0
            - t * t * t * t / 65194000.0,
    )
}

/// Moon mean elongation (D), degrees
fn moon_mean_elongation(t: f64) -> f64 {
    normalize_degrees(
        297.8501921 + 445267.1114034 * t
            - 0.0018819 * t * t
            + t * t * t / 545868.0
            - t * t * t * t / 113065000.0,
    )
}

/// Sun mean anomaly (M), degrees
fn sun_mean_anomaly(t: f64) -> f64 {
    normalize_degrees(
        357.5291092 + 35999.0502909 * t
            - 0.0001536 * t * t
            + t * t * t / 24490000.0,
    )
}

/// Moon mean anomaly (M'), degrees
fn moon_mean_anomaly(t: f64) -> f64 {
    normalize_degrees(
        134.9633964 + 477198.8675055 * t
            + 0.0087414 * t * t
            + t * t * t / 69699.0
            - t * t * t * t / 14712000.0,
    )
}

/// Moon argument of latitude (F), degrees
fn moon_argument_of_latitude(t: f64) -> f64 {
    normalize_degrees(
        93.2720950 + 483202.0175233 * t
            - 0.0036539 * t * t
            - t * t * t / 3526000.0
            + t * t * t * t / 863310000.0,
    )
}

/// Compute ecliptic coordinates of the Moon.
/// Returns (longitude_deg, latitude_deg, distance_km).
fn moon_ecliptic(t: f64) -> (f64, f64, f64) {
    let lp = moon_mean_longitude(t);
    let d = moon_mean_elongation(t);
    let m = sun_mean_anomaly(t);
    let mp = moon_mean_anomaly(t);
    let f = moon_argument_of_latitude(t);

    // Earth eccentricity correction
    let e = 1.0 - 0.002516 * t - 0.0000074 * t * t;
    let e2 = e * e;

    let mut sum_l: f64 = 0.0;
    let mut sum_r: f64 = 0.0;

    for &(td, tm, tmp, tf, cl, cr) in &TERMS_LR {
        let arg = (td * d + tm * m + tmp * mp + tf * f) * DEG;
        let m_abs = tm.abs() as i32;
        let e_factor = if m_abs == 1 { e } else if m_abs == 2 { e2 } else { 1.0 };
        sum_l += cl * e_factor * arg.sin();
        sum_r += cr * e_factor * arg.cos();
    }

    let mut sum_b: f64 = 0.0;
    for &(td, tm, tmp, tf, cb) in &TERMS_B {
        let arg = (td * d + tm * m + tmp * mp + tf * f) * DEG;
        let m_abs = tm.abs() as i32;
        let e_factor = if m_abs == 1 { e } else if m_abs == 2 { e2 } else { 1.0 };
        sum_b += cb * e_factor * arg.sin();
    }

    // Additive corrections (A1, A2, A3)
    let a1 = normalize_degrees(119.75 + 131.849 * t);
    let a2 = normalize_degrees(53.09 + 479264.290 * t);
    let a3 = normalize_degrees(313.45 + 481266.484 * t);

    sum_l += 3958.0 * (a1 * DEG).sin();
    sum_l += 1962.0 * ((lp - f) * DEG).sin();
    sum_l += 318.0 * (a2 * DEG).sin();

    sum_b += -2235.0 * (lp * DEG).sin();
    sum_b += 382.0 * (a3 * DEG).sin();
    sum_b += 175.0 * ((a1 - f) * DEG).sin();
    sum_b += 175.0 * ((a1 + f) * DEG).sin();
    sum_b += 127.0 * ((lp - mp) * DEG).sin();
    sum_b += -115.0 * ((lp + mp) * DEG).sin();

    let longitude = normalize_degrees(lp + sum_l / 1_000_000.0);
    let latitude = sum_b / 1_000_000.0;
    let distance = 385000.56 + sum_r / 1000.0;

    (longitude, latitude, distance)
}

/// Local sidereal time in degrees for a given JD and longitude.
fn local_sidereal_time(jd: f64, lon: f64) -> f64 {
    let t = julian_century(jd);
    let gmst = normalize_degrees(
        280.46061837 + 360.98564736629 * (jd - 2451545.0)
            + 0.000387933 * t * t
            - t * t * t / 38710000.0,
    );
    normalize_degrees(gmst + lon)
}

/// Ecliptic to equatorial coordinate transform.
/// Returns (right_ascension_deg, declination_deg).
fn ecliptic_to_equatorial(lon: f64, lat: f64, obliquity: f64) -> (f64, f64) {
    let lon_r = lon * DEG;
    let lat_r = lat * DEG;
    let obl_r = obliquity * DEG;

    let sin_ra = lon_r.sin() * obl_r.cos() - lat_r.tan() * obl_r.sin();
    let cos_ra = lon_r.cos();
    let ra = normalize_degrees(sin_ra.atan2(cos_ra) / DEG);

    let sin_dec = lat_r.sin() * obl_r.cos() + lat_r.cos() * obl_r.sin() * lon_r.sin();
    let dec = sin_dec.asin() / DEG;

    (ra, dec)
}

/// Equatorial to horizontal coordinate transform.
/// Returns (altitude_deg, azimuth_deg).
fn equatorial_to_horizontal(ra: f64, dec: f64, lat: f64, lst: f64) -> (f64, f64) {
    let ha = normalize_degrees(lst - ra) * DEG;
    let dec_r = dec * DEG;
    let lat_r = lat * DEG;

    let sin_alt = lat_r.sin() * dec_r.sin() + lat_r.cos() * dec_r.cos() * ha.cos();
    let alt = sin_alt.asin() / DEG;

    let cos_az = (dec_r.sin() - sin_alt * lat_r.sin()) / (sin_alt.asin().cos() * lat_r.cos());
    let az = cos_az.clamp(-1.0, 1.0).acos() / DEG;
    let azimuth = if ha.sin() > 0.0 { 360.0 - az } else { az };

    (alt, azimuth)
}

/// Apply topocentric parallax correction to the Moon's altitude.
/// The Moon's horizontal parallax is approximately asin(6378.14 / distance_km).
fn topocentric_correction(geo_alt: f64, distance_km: f64, observer_lat: f64) -> f64 {
    let hp = (6378.14 / distance_km).asin(); // horizontal parallax in radians
    let alt_r = geo_alt * DEG;
    let _lat_r = observer_lat * DEG;
    // Simplified parallax in altitude
    let parallax = hp * alt_r.cos();
    geo_alt - parallax / DEG
}

/// Apply atmospheric refraction correction.
fn refraction_correction(apparent_alt: f64) -> f64 {
    if apparent_alt < -1.0 {
        return apparent_alt;
    }
    // Bennett's formula
    let r = 1.02 / ((apparent_alt + 10.3 / (apparent_alt + 5.11)) * DEG).tan();
    apparent_alt + r / 60.0
}

/// Compute the full lunar position for a given UTC datetime and observer location.
pub fn lunar_position(dt: &NaiveDateTime, lat: f64, lon: f64) -> LunarPosition {
    let jd = julian_date(dt);
    let t = julian_century(jd);

    let (moon_lon, moon_lat, distance) = moon_ecliptic(t);
    let obliquity = obliquity_corrected(t);
    let (ra, dec) = ecliptic_to_equatorial(moon_lon, moon_lat, obliquity);

    let lst = local_sidereal_time(jd, lon);
    let (geo_alt, azimuth) = equatorial_to_horizontal(ra, dec, lat, lst);

    // Apply topocentric parallax (significant for the Moon, ~0.95°)
    let topo_alt = topocentric_correction(geo_alt, distance, lat);

    // Apply atmospheric refraction
    let altitude = refraction_correction(topo_alt);

    LunarPosition {
        longitude: moon_lon,
        latitude: moon_lat,
        distance_km: distance,
        right_ascension: ra,
        declination: dec,
        altitude,
        azimuth,
    }
}

/// Compute the Moon-Sun elongation (angular separation) at a given UTC datetime.
/// Returns elongation in degrees (0° at conjunction, ~180° at full moon).
pub fn moon_sun_elongation(dt: &NaiveDateTime) -> f64 {
    let jd = julian_date(dt);
    let t = julian_century(jd);

    let (moon_lon, moon_lat, _) = moon_ecliptic(t);
    let sun_lon = sun_ecliptic_longitude(dt);

    let d_lon = (moon_lon - sun_lon) * DEG;
    let moon_lat_r = moon_lat * DEG;

    // Elongation via spherical geometry
    let cos_elong = moon_lat_r.cos() * d_lon.cos();
    cos_elong.clamp(-1.0, 1.0).acos() / DEG
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_meeus_example_47a() {
        // Meeus Example 47.a: 1992 April 12, 0h TD
        let dt = NaiveDate::from_ymd_opt(1992, 4, 12)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        let jd = julian_date(&dt);
        let t = julian_century(jd);
        let (lon, lat, dist) = moon_ecliptic(t);

        // Expected: longitude ~133.17°, latitude ~-3.23°, distance ~368409 km
        assert!(
            (lon - 133.17).abs() < 0.5,
            "Moon longitude: expected ~133.17°, got {:.2}°",
            lon
        );
        assert!(
            (lat - (-3.23)).abs() < 0.5,
            "Moon latitude: expected ~-3.23°, got {:.2}°",
            lat
        );
        assert!(
            (dist - 368409.0).abs() < 2000.0,
            "Moon distance: expected ~368409 km, got {:.0} km",
            dist
        );
    }

    #[test]
    fn test_conjunction_feb17_2026() {
        // On Feb 17, 2026 around 12:00 UTC, new moon conjunction occurs.
        // Elongation should be very small (< 10°).
        let dt = NaiveDate::from_ymd_opt(2026, 2, 17)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        let elong = moon_sun_elongation(&dt);
        assert!(
            elong < 10.0,
            "Elongation at conjunction should be < 10°, got {:.2}°",
            elong
        );
    }

    #[test]
    fn test_full_moon_elongation() {
        // Full moon ~ elongation near 180°
        // Jan 13, 2025 is approximately a full moon
        let dt = NaiveDate::from_ymd_opt(2025, 1, 13)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        let elong = moon_sun_elongation(&dt);
        assert!(
            elong > 160.0,
            "Elongation at full moon should be > 160°, got {:.2}°",
            elong
        );
    }

    #[test]
    fn test_lunar_position_mecca() {
        // Basic sanity: altitude should be between -90 and 90
        let dt = NaiveDate::from_ymd_opt(2026, 2, 18)
            .unwrap()
            .and_hms_opt(15, 30, 0)
            .unwrap();
        let pos = lunar_position(&dt, 21.4225, 39.8262);
        assert!(pos.altitude >= -90.0 && pos.altitude <= 90.0);
        assert!(pos.azimuth >= 0.0 && pos.azimuth <= 360.0);
        assert!(pos.distance_km > 350000.0 && pos.distance_km < 410000.0);
    }
}
