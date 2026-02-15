//! Core types for the location subsystem.

use serde::{Deserialize, Serialize};
use std::fmt;

/// How a location was resolved.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LocationSource {
    Cache,
    Nominatim,
    IpApi,
    Fallback,
    Manual,
}

impl fmt::Display for LocationSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cache => write!(f, "Cache"),
            Self::Nominatim => write!(f, "Nominatim"),
            Self::IpApi => write!(f, "IP"),
            Self::Fallback => write!(f, "Built-in"),
            Self::Manual => write!(f, "Manual"),
        }
    }
}

/// A fully resolved location with coordinates, timezone, and provenance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedLocation {
    pub name: String,
    pub lat: f64,
    pub lon: f64,
    pub tz: String,
    pub source: LocationSource,
    /// Full display name from provider (e.g. "Medina, Al Madinah, Saudi Arabia")
    #[serde(default)]
    pub display_name: Option<String>,
    /// ISO 3166-1 alpha-2 country code (e.g. "SA", "US")
    #[serde(default)]
    pub country_code: Option<String>,
    /// Resolver confidence score (0.0 to 1.0)
    #[serde(default = "default_confidence")]
    pub resolver_confidence: f64,
    /// Whether disambiguation was applied
    #[serde(default)]
    pub disambiguated: bool,
    /// Human-readable disambiguation note
    #[serde(default)]
    pub disambiguation_note: Option<String>,
}

fn default_confidence() -> f64 {
    1.0
}

impl ResolvedLocation {
    pub fn display_line(&self) -> String {
        let country_part = match &self.country_code {
            Some(cc) => {
                let name = super::providers::country_display_name(cc);
                format!(" \u{2014} {}", name)
            }
            None => String::new(),
        };
        let coords = super::providers::format_coords(self.lat, self.lon);
        format!(
            "\u{1F4CD} {}{}\n  \u{1F552} {} (Local Time)\n  \u{1F4D0} {}",
            self.name, country_part, self.tz, coords
        )
    }
}

/// Options for city resolution.
#[derive(Debug, Clone, Default)]
pub struct ResolveOptions {
    /// ISO 3166-1 alpha-2 country code hint (e.g. "SA")
    pub country: Option<String>,
    /// Show top-K candidates (debug mode)
    pub topk: Option<usize>,
}

/// Location resolution errors.
#[derive(Debug)]
pub enum LocationError {
    Network(String),
    NotFound(String),
    CacheMiss,
    InvalidResponse(String),
    NoInput,
    /// Ambiguous city name — multiple strong candidates exist.
    Ambiguous {
        query: String,
        candidates: Vec<AmbiguousCandidate>,
    },
}

/// A candidate shown to the user when disambiguation fails.
#[derive(Debug, Clone, Serialize)]
pub struct AmbiguousCandidate {
    pub name: String,
    pub country: String,
    pub country_name: String,
    pub lat: f64,
    pub lon: f64,
    pub tz: String,
    pub score: f64,
}

impl fmt::Display for LocationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Network(msg) => write!(f, "Network error: {}", msg),
            Self::NotFound(q) => write!(f, "Location not found: '{}'", q),
            Self::CacheMiss => write!(f, "No cached location available"),
            Self::InvalidResponse(msg) => write!(f, "Invalid API response: {}", msg),
            Self::NoInput => write!(f, "No location specified. Use --city, --auto, or --lat/--lon"),
            Self::Ambiguous { query, candidates } => {
                writeln!(f, "Ambiguous city name: '{}'", query)?;
                writeln!(f)?;
                writeln!(f, "  Multiple matches found:")?;
                for (i, c) in candidates.iter().enumerate().take(5) {
                    let coords = super::providers::format_coords(c.lat, c.lon);
                    writeln!(f, "    {}. \u{1F4CD} {} \u{2014} {}", i + 1, c.name, c.country_name)?;
                    writeln!(f, "       \u{1F552} {} (Local Time)", c.tz)?;
                    writeln!(f, "       \u{1F4D0} {}", coords)?;
                }
                writeln!(f)?;
                write!(f, "  Hint: Try --city \"{}, {}\" or --country {}", query, candidates[0].country, candidates[0].country)
            }
        }
    }
}

impl std::error::Error for LocationError {}
