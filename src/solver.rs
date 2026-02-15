//! The Solver — primary public API for Polaris Chronos.
//!
//! Handles timezone conversion, current state detection,
//! wave debug output, and ASCII visualization.

use crate::location::{LocationSource, ResolvedLocation, country_display_name, format_coords};
use crate::schedule::{self, DayState, Events, EventMethod, GapStrategy, PrayerEvent};
use crate::solar;
use chrono::{NaiveDate, Timelike, Utc, FixedOffset, Offset};
use chrono_tz::Tz;
use serde::Serialize;

/// Location input (legacy, still usable for direct lat/lon).
#[derive(Debug, Clone, Copy)]
pub struct Location {
    pub lat: f64,
    pub lon: f64,
}

impl Location {
    pub fn new(lat: f64, lon: f64) -> Self {
        assert!((-90.0..=90.0).contains(&lat), "Latitude must be between -90 and 90");
        assert!((-180.0..=180.0).contains(&lon), "Longitude must be between -180 and 180");
        Self { lat, lon }
    }
}

/// Full solver output.
#[derive(Debug, Clone, Serialize)]
pub struct SolverOutput {
    pub location: LocationInfo,
    pub date: String,
    pub state: DayState,
    pub gap_strategy: GapStrategy,
    pub events: Events,
    pub solar: schedule::SolarInfo,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current: Option<CurrentState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wave_debug: Option<WaveDebug>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LocationInfo {
    pub name: String,
    pub latitude: f64,
    pub longitude: f64,
    pub timezone: String,
    pub tz_label: String,
    pub source: LocationSource,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    pub formatted_coords: String,
    pub resolved_confidence: f64,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub disambiguated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disambiguation_note: Option<String>,
}

/// Current prayer state (--now mode).
#[derive(Debug, Clone, Serialize)]
pub struct CurrentState {
    pub prayer: String,
    pub next: String,
    pub remaining_minutes: i64,
}

/// Wave debug data (--debug-wave mode).
#[derive(Debug, Clone, Serialize)]
pub struct WaveDebug {
    pub sample_count: usize,
    pub peak_index: usize,
    pub nadir_index: usize,
    /// Compressed altitude curve: one sample per 10 minutes.
    pub altitudes: Vec<f64>,
}

/// The Solver.
pub struct Solver {
    location: Location,
    tz: Tz,
    strategy: GapStrategy,
}

impl Solver {
    pub fn new(location: Location, tz: Tz) -> Self {
        Self { location, tz, strategy: GapStrategy::default() }
    }

    pub fn with_utc(location: Location) -> Self {
        Self { location, tz: chrono_tz::UTC, strategy: GapStrategy::default() }
    }

    /// Create a solver from a ResolvedLocation.
    pub fn from_resolved(resolved: &ResolvedLocation) -> Self {
        let tz: Tz = resolved.tz.parse().unwrap_or(chrono_tz::UTC);
        Self {
            location: Location::new(resolved.lat, resolved.lon),
            tz,
            strategy: GapStrategy::default(),
        }
    }

    /// Set the gap strategy for polar event handling.
    pub fn with_strategy(mut self, strategy: GapStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    pub fn solve(&self, date: NaiveDate, now_mode: bool, debug_wave: bool) -> SolverOutput {
        self.solve_with_info(date, now_mode, debug_wave, None)
    }

    /// Solve with full location metadata from a ResolvedLocation.
    pub fn solve_with_info(
        &self,
        date: NaiveDate,
        now_mode: bool,
        debug_wave: bool,
        resolved: Option<&ResolvedLocation>,
    ) -> SolverOutput {
        let schedule = schedule::compute_schedule(date, self.location.lat, self.location.lon, self.strategy);

        let tz_name = self.tz.to_string();
        let utc_offset_secs = self.utc_offset_seconds(date);

        let events = self.convert_events(&schedule.events, utc_offset_secs);

        let current = if now_mode {
            self.detect_current(&events, utc_offset_secs)
        } else {
            None
        };

        let wave_debug = if debug_wave {
            Some(self.build_wave_debug(date))
        } else {
            None
        };

        let location_info = match resolved {
            Some(r) => LocationInfo {
                name: r.name.clone(),
                latitude: r.lat,
                longitude: r.lon,
                timezone: tz_name.clone(),
                tz_label: format!("{} (Local Time)", tz_name),
                source: r.source.clone(),
                country_code: r.country_code.clone(),
                country: r.country_code.as_deref().and_then(|cc| {
                    let name = country_display_name(cc);
                    if name == cc { None } else { Some(name.to_string()) }
                }),
                formatted_coords: format_coords(r.lat, r.lon),
                resolved_confidence: r.resolver_confidence,
                disambiguated: r.disambiguated,
                disambiguation_note: r.disambiguation_note.clone(),
            },
            None => LocationInfo {
                name: format!("{:.4}, {:.4}", self.location.lat, self.location.lon),
                latitude: self.location.lat,
                longitude: self.location.lon,
                timezone: tz_name.clone(),
                tz_label: format!("{} (Local Time)", tz_name),
                source: LocationSource::Manual,
                country_code: None,
                country: None,
                formatted_coords: format_coords(self.location.lat, self.location.lon),
                resolved_confidence: 1.0,
                disambiguated: false,
                disambiguation_note: None,
            },
        };

        SolverOutput {
            location: location_info,
            date: date.to_string(),
            state: schedule.state,
            gap_strategy: self.strategy,
            events,
            solar: schedule.solar,
            current,
            wave_debug,
        }
    }

    /// Get UTC offset in seconds for a given date at this timezone.
    fn utc_offset_seconds(&self, date: NaiveDate) -> i64 {
        use chrono::TimeZone;
        let noon = date.and_hms_opt(12, 0, 0).unwrap();
        match self.tz.from_local_datetime(&noon).earliest() {
            Some(dt) => {
                let fixed: FixedOffset = dt.offset().fix();
                fixed.local_minus_utc() as i64
            }
            None => 0,
        }
    }

    /// Convert events from UTC to local time.
    fn convert_events(&self, events: &Events, offset_secs: i64) -> Events {
        Events {
            fajr: self.convert_event(&events.fajr, offset_secs),
            sunrise: self.convert_event(&events.sunrise, offset_secs),
            dhuhr: self.convert_event(&events.dhuhr, offset_secs),
            asr: self.convert_event(&events.asr, offset_secs),
            maghrib: self.convert_event(&events.maghrib, offset_secs),
            isha: self.convert_event(&events.isha, offset_secs),
        }
    }

    fn convert_event(&self, event: &PrayerEvent, offset_secs: i64) -> PrayerEvent {
        let mut next_day = false;
        let time = event.time.as_ref().map(|t| {
            let utc_secs = hms_to_secs(t);
            let local_secs = utc_secs + offset_secs as f64;
            if local_secs >= 86400.0 {
                next_day = true;
            }
            solar::seconds_to_hms(local_secs)
        });

        // Append "(next day)" to note when wrapping occurs
        let note = if next_day {
            match &event.note {
                Some(n) => Some(format!("{} (next day)", n)),
                None => Some("next day".to_string()),
            }
        } else {
            event.note.clone()
        };

        PrayerEvent {
            time,
            method: event.method,
            confidence: event.confidence,
            note,
            next_day,
        }
    }

    /// Detect current and next prayer based on current UTC time.
    fn detect_current(&self, local_events: &Events, offset_secs: i64) -> Option<CurrentState> {
        let now_utc = Utc::now().naive_utc();
        let now_local_secs = (now_utc.hour() as f64 * 3600.0
            + now_utc.minute() as f64 * 60.0
            + now_utc.second() as f64)
            + offset_secs as f64;
        let now_local_secs = ((now_local_secs % 86400.0) + 86400.0) % 86400.0;

        let prayer_list = [
            ("Fajr", &local_events.fajr),
            ("Sunrise", &local_events.sunrise),
            ("Dhuhr", &local_events.dhuhr),
            ("Asr", &local_events.asr),
            ("Maghrib", &local_events.maghrib),
            ("Isha", &local_events.isha),
        ];

        // Collect events that have a time
        let timed: Vec<(&str, f64)> = prayer_list
            .iter()
            .filter_map(|(name, ev)| {
                ev.time.as_ref().map(|t| (*name, hms_to_secs(t)))
            })
            .collect();

        if timed.is_empty() {
            return None;
        }

        // Find current period
        let mut current_prayer = timed.last().unwrap().0;
        let mut next_prayer = timed.first().unwrap().0;
        let mut next_secs = timed.first().unwrap().1 + 86400.0; // tomorrow

        for i in 0..timed.len() {
            if now_local_secs < timed[i].1 {
                next_prayer = timed[i].0;
                next_secs = timed[i].1;
                if i > 0 {
                    current_prayer = timed[i - 1].0;
                } else {
                    current_prayer = timed.last().unwrap().0;
                }
                break;
            }
            if i == timed.len() - 1 {
                current_prayer = timed[i].0;
                next_prayer = timed[0].0;
                next_secs = timed[0].1 + 86400.0;
            }
        }

        let remaining = ((next_secs - now_local_secs) / 60.0).ceil() as i64;

        Some(CurrentState {
            prayer: current_prayer.to_string(),
            next: next_prayer.to_string(),
            remaining_minutes: remaining.max(0),
        })
    }

    fn build_wave_debug(&self, date: NaiveDate) -> WaveDebug {
        let samples = schedule::day_scan_samples(date, self.location.lat, self.location.lon);
        let peak_idx = samples.iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.altitude.partial_cmp(&b.altitude).unwrap())
            .map(|(i, _)| i)
            .unwrap_or(0);
        let nadir_idx = samples.iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| a.altitude.partial_cmp(&b.altitude).unwrap())
            .map(|(i, _)| i)
            .unwrap_or(0);

        // Compress: pick every 20th sample (30-sec resolution → ~10 min intervals)
        let altitudes: Vec<f64> = samples.iter()
            .step_by(20)
            .map(|s| (s.altitude * 100.0).round() / 100.0)
            .collect();

        WaveDebug {
            sample_count: samples.len(),
            peak_index: peak_idx,
            nadir_index: nadir_idx,
            altitudes,
        }
    }
}

fn hms_to_secs(hms: &str) -> f64 {
    let parts: Vec<&str> = hms.split(':').collect();
    if parts.len() != 3 { return 0.0; }
    let h: f64 = parts[0].parse().unwrap_or(0.0);
    let m: f64 = parts[1].parse().unwrap_or(0.0);
    let s: f64 = parts[2].parse().unwrap_or(0.0);
    h * 3600.0 + m * 60.0 + s
}

// ─── ASCII Visualization ────────────────────────────────────────

pub fn render_ascii_timeline(events: &Events, state: DayState, strategy: GapStrategy, show_confidence: bool) -> String {
    let mut out = String::new();

    // Header
    if state != DayState::Normal {
        out.push_str(&format!("  Solar Day: {:?} (Gap Strategy: {})\n", state, strategy));
    } else {
        out.push_str(&format!("  Solar Day: {:?}\n", state));
    }
    out.push_str("  ╔══════════════════════════════════════════════════════════════╗\n");

    // Build the event list with times
    let items: Vec<(&str, &PrayerEvent, &str)> = vec![
        ("Fajr    ", &events.fajr, "░"),
        ("Sunrise ", &events.sunrise, "▓"),
        ("Dhuhr   ", &events.dhuhr, "█"),
        ("Asr     ", &events.asr, "▓"),
        ("Maghrib ", &events.maghrib, "░"),
        ("Isha    ", &events.isha, " "),
    ];

    // Timeline bar (60 chars = 24 hours)
    let bar_width = 60;
    let mut markers: Vec<(usize, &str)> = Vec::new();

    for (label, event, _sym) in &items {
        if let Some(ref t) = event.time {
            let secs = hms_to_secs(t);
            let pos = ((secs / 86400.0) * bar_width as f64) as usize;
            let pos = pos.min(bar_width - 1);
            markers.push((pos, label.trim()));
        }
    }

    // Draw timeline
    let mut bar = vec!['─'; bar_width];
    for (pos, _) in &markers {
        bar[*pos] = '│';
    }
    out.push_str("  ║ ");
    out.push_str(&bar.iter().collect::<String>());
    out.push_str(" ║\n");

    // Draw labels below
    let mut label_line = vec![' '; bar_width];
    for (pos, name) in &markers {
        let first = name.chars().next().unwrap_or('?');
        if *pos < bar_width {
            label_line[*pos] = first;
        }
    }
    out.push_str("  ║ ");
    out.push_str(&label_line.iter().collect::<String>());
    out.push_str(" ║\n");

    out.push_str("  ╠══════════════════════════════════════════════════════════════╣\n");

    // Event list
    for (label, event, _) in &items {
        let time_str = match &event.time {
            Some(t) => {
                if event.next_day {
                    format!("{} (+1d)", t)
                } else {
                    t.clone()
                }
            }
            None => "────────".to_string(),
        };
        let method_tag = match event.method {
            EventMethod::Standard => "",
            EventMethod::Virtual => " [V]",
            EventMethod::Projected => " [P]",
            EventMethod::None => " [N/A]",
        };
        let conf_tag = if show_confidence && event.method != EventMethod::Standard {
            format!(" ({:.1})", event.confidence)
        } else {
            String::new()
        };
        out.push_str(&format!("  ║  {} {}{}{}",
            label.trim(),
            time_str,
            method_tag,
            conf_tag,
        ));
        // Pad to fixed width
        let line_len = 4 + label.trim().len() + 1 + time_str.len() + method_tag.len() + conf_tag.len();
        let pad = if 64 > line_len { 64 - line_len } else { 1 };
        out.push_str(&" ".repeat(pad));
        out.push_str("║\n");
    }

    out.push_str("  ╚══════════════════════════════════════════════════════════════╝\n");
    out.push_str("  00:00          06:00          12:00          18:00       23:59\n");

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schedule::DayState;
    use chrono::NaiveDate;

    fn utc_solver(lat: f64, lon: f64) -> Solver {
        Solver::with_utc(Location::new(lat, lon))
    }

    #[test]
    fn test_solver_mecca_normal() {
        let solver = utc_solver(21.4225, 39.8262);
        let output = solver.solve(NaiveDate::from_ymd_opt(2026, 2, 14).unwrap(), false, false);

        println!("{}", serde_json::to_string_pretty(&output).unwrap());

        assert_eq!(output.state, DayState::Normal);
        assert!(output.events.sunrise.time.is_some());
        assert!(output.events.maghrib.time.is_some());
        assert_eq!(output.events.sunrise.method, EventMethod::Standard);
    }

    #[test]
    fn test_solver_polar_night_truthful() {
        let solver = utc_solver(78.2232, 15.6267).with_strategy(GapStrategy::Strict);
        let output = solver.solve(NaiveDate::from_ymd_opt(2025, 12, 21).unwrap(), false, false);

        assert_eq!(output.state, DayState::PolarNight);
        assert!(output.events.sunrise.time.is_none(), "PolarNight: sunrise must be None");
        assert!(output.events.maghrib.time.is_none(), "PolarNight: maghrib must be None");
        assert_eq!(output.events.sunrise.method, EventMethod::None);
        assert_eq!(output.events.maghrib.method, EventMethod::None);
    }

    #[test]
    fn test_solver_midnight_sun_truthful() {
        let solver = utc_solver(69.6492, 18.9553).with_strategy(GapStrategy::Strict);
        let output = solver.solve(NaiveDate::from_ymd_opt(2026, 6, 21).unwrap(), false, false);

        assert_eq!(output.state, DayState::MidnightSun);
        assert!(output.events.sunrise.time.is_none(), "MidnightSun: sunrise must be None");
        assert!(output.events.maghrib.time.is_none(), "MidnightSun: maghrib must be None");
    }

    #[test]
    fn test_timezone_conversion() {
        // Mecca is UTC+3 (Asia/Riyadh)
        let tz: Tz = "Asia/Riyadh".parse().unwrap();
        let solver = Solver::new(Location::new(21.4225, 39.8262), tz);
        let output = solver.solve(NaiveDate::from_ymd_opt(2026, 2, 14).unwrap(), false, false);

        assert_eq!(output.location.timezone, "Asia/Riyadh");

        // Dhuhr in UTC was ~09:35, in Riyadh should be ~12:35
        let dhuhr = output.events.dhuhr.time.as_ref().unwrap();
        assert!(dhuhr.starts_with("12:"), "Dhuhr in Riyadh should be around 12:xx, got {}", dhuhr);
    }

    #[test]
    fn test_wave_debug() {
        let solver = utc_solver(78.2232, 15.6267);
        let output = solver.solve(NaiveDate::from_ymd_opt(2025, 12, 21).unwrap(), false, true);

        let wave = output.wave_debug.as_ref().unwrap();
        assert!(wave.sample_count > 1000); // 30-sec resolution → 2880 samples
        assert!(!wave.altitudes.is_empty());
        // All altitudes should be negative for polar night
        assert!(wave.altitudes.iter().all(|a| *a < 0.0));
    }

    #[test]
    fn test_ascii_timeline() {
        let solver = utc_solver(21.4225, 39.8262);
        let output = solver.solve(NaiveDate::from_ymd_opt(2026, 2, 14).unwrap(), false, false);
        let ascii = render_ascii_timeline(&output.events, output.state, output.gap_strategy, false);
        println!("{}", ascii);
        assert!(ascii.contains("Fajr"));
        assert!(ascii.contains("Dhuhr"));
        assert!(ascii.contains("Isha"));
    }

    #[test]
    fn test_ascii_timeline_polar_night() {
        let solver = utc_solver(78.2232, 15.6267).with_strategy(GapStrategy::Strict);
        let output = solver.solve(NaiveDate::from_ymd_opt(2025, 12, 21).unwrap(), false, false);
        let ascii = render_ascii_timeline(&output.events, output.state, output.gap_strategy, false);
        println!("{}", ascii);
        assert!(ascii.contains("[N/A]"));
        assert!(ascii.contains("[V]"));
    }

    #[test]
    #[should_panic(expected = "Latitude must be between")]
    fn test_invalid_latitude() {
        Location::new(91.0, 0.0);
    }

    #[test]
    fn test_three_cities_integration() {
        let cases = vec![
            ("Mecca", 21.4225, 39.8262, "2026-02-14", DayState::Normal),
            ("Tromsø", 69.6492, 18.9553, "2026-02-14", DayState::Normal),
            ("Svalbard", 78.2232, 15.6267, "2025-12-21", DayState::PolarNight),
        ];

        for (name, lat, lon, date_str, expected) in cases {
            let solver = utc_solver(lat, lon).with_strategy(GapStrategy::Strict);
            let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d").unwrap();
            let output = solver.solve(date, false, false);

            println!("--- {} ({}) ---", name, date_str);
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
            println!("{}", render_ascii_timeline(&output.events, output.state, output.gap_strategy, false));

            assert_eq!(output.state, expected, "{} state mismatch", name);

            // Truthfulness checks (Strict mode)
            match expected {
                DayState::PolarNight | DayState::MidnightSun => {
                    assert!(output.events.sunrise.time.is_none(),
                        "{}: sunrise must be None in {:?}", name, expected);
                    assert!(output.events.maghrib.time.is_none(),
                        "{}: maghrib must be None in {:?}", name, expected);
                }
                DayState::Normal => {
                    assert!(output.events.sunrise.time.is_some());
                    assert!(output.events.maghrib.time.is_some());
                }
            }
        }
    }

    // ─── v6 Solver Projection Tests ──────────────────────────────

    #[test]
    fn test_solver_projected_output_has_strategy() {
        let solver = utc_solver(69.6492, 18.9553);  // default = Projected45
        let output = solver.solve(NaiveDate::from_ymd_opt(2026, 6, 21).unwrap(), false, false);

        let json = serde_json::to_string_pretty(&output).unwrap();
        assert!(json.contains("gap_strategy"), "JSON must include gap_strategy field");
        assert!(json.contains("Projected45"), "Default strategy should be Projected45");
    }

    #[test]
    fn test_solver_strict_mode() {
        let solver = utc_solver(78.2232, 15.6267).with_strategy(GapStrategy::Strict);
        let output = solver.solve(NaiveDate::from_ymd_opt(2025, 12, 21).unwrap(), false, false);

        assert_eq!(output.gap_strategy, GapStrategy::Strict);
        assert!(output.events.sunrise.time.is_none());
        assert!(output.events.maghrib.time.is_none());
    }

    #[test]
    fn test_ascii_timeline_shows_projected() {
        let solver = utc_solver(69.6492, 18.9553);  // default Projected45
        let output = solver.solve(NaiveDate::from_ymd_opt(2026, 6, 21).unwrap(), false, false);
        let ascii = render_ascii_timeline(&output.events, output.state, output.gap_strategy, false);
        println!("{}", ascii);
        assert!(ascii.contains("[P]"), "Timeline must show [P] tag");
        assert!(ascii.contains("Projected45"), "Header must show strategy name");
    }

    // ─── v6.2 Production Upgrade Tests ──────────────────────────

    #[test]
    fn test_confidence_in_json_output() {
        let solver = utc_solver(78.2232, 15.6267);
        let output = solver.solve(NaiveDate::from_ymd_opt(2025, 12, 21).unwrap(), false, false);
        let json = serde_json::to_string_pretty(&output).unwrap();
        assert!(json.contains("\"confidence\""), "JSON must include confidence field");
        assert!(json.contains("0.5"), "JSON must show 0.5 for projected events");
        assert!(json.contains("0.7"), "JSON must show 0.7 for virtual events");
    }

    #[test]
    fn test_date_wrapping_next_day() {
        // Use a timezone where late UTC events wrap past midnight local time
        // Tromsø projected isha in midnight sun can be very late UTC;
        // use a far-east timezone to force wrapping
        let tz: Tz = "Pacific/Auckland".parse().unwrap(); // UTC+12/+13
        let solver = Solver::new(Location::new(21.4225, 39.8262), tz);
        let output = solver.solve(NaiveDate::from_ymd_opt(2026, 2, 14).unwrap(), false, false);

        // Isha in Mecca UTC ~16:28 + 13h offset = ~05:28 next day
        let isha = &output.events.isha;
        assert!(isha.next_day, "Isha in Auckland TZ should wrap to next day");
        assert!(isha.note.is_some(), "Wrapped event must have note");
        assert!(isha.note.as_ref().unwrap().contains("next day"),
            "Note must contain 'next day', got: {:?}", isha.note);
    }

    #[test]
    fn test_date_wrapping_cli_display() {
        let tz: Tz = "Pacific/Auckland".parse().unwrap();
        let solver = Solver::new(Location::new(21.4225, 39.8262), tz);
        let output = solver.solve(NaiveDate::from_ymd_opt(2026, 2, 14).unwrap(), false, false);
        let ascii = render_ascii_timeline(&output.events, output.state, output.gap_strategy, false);
        println!("{}", ascii);
        assert!(ascii.contains("(+1d)"), "CLI must show (+1d) for wrapped events");
    }

    #[test]
    fn test_show_confidence_flag() {
        let solver = utc_solver(78.2232, 15.6267);
        let output = solver.solve(NaiveDate::from_ymd_opt(2025, 12, 21).unwrap(), false, false);

        // Without show_confidence
        let ascii_no = render_ascii_timeline(&output.events, output.state, output.gap_strategy, false);
        assert!(!ascii_no.contains("(0.7)"), "Should NOT show confidence without flag");

        // With show_confidence
        let ascii_yes = render_ascii_timeline(&output.events, output.state, output.gap_strategy, true);
        assert!(ascii_yes.contains("(0.7)"), "Should show confidence with flag");
        assert!(ascii_yes.contains("(0.5)"), "Should show projected confidence");
    }

    #[test]
    fn test_short_tags_in_timeline() {
        let solver = utc_solver(78.2232, 15.6267).with_strategy(GapStrategy::Strict);
        let output = solver.solve(NaiveDate::from_ymd_opt(2025, 12, 21).unwrap(), false, false);
        let ascii = render_ascii_timeline(&output.events, output.state, output.gap_strategy, false);
        assert!(ascii.contains("[V]"), "Virtual events should use [V] short tag");
        assert!(ascii.contains("[N/A]"), "None events should still show [N/A]");
        // Long tags should NOT appear
        assert!(!ascii.contains("[Virtual]"), "[Virtual] long tag should not appear");
    }
}
