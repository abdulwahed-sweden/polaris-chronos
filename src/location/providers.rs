//! Location providers: Nominatim, IP API, and built-in fallback dataset.

use super::types::{LocationError, LocationSource, ResolvedLocation};
use serde::{Deserialize, Serialize};

// ─── Built-in dataset ───────────────────────────────────────────

struct BuiltinCity {
    names: &'static [&'static str], // canonical + aliases
    lat: f64,
    lon: f64,
    tz: &'static str,
    country_code: &'static str,
}

const BUILTIN_CITIES: &[BuiltinCity] = &[
    BuiltinCity {
        names: &["mecca", "makkah", "mekka"],
        lat: 21.4225, lon: 39.8262, tz: "Asia/Riyadh",
        country_code: "SA",
    },
    BuiltinCity {
        names: &["medina", "madinah", "al-madinah"],
        lat: 24.4686, lon: 39.6142, tz: "Asia/Riyadh",
        country_code: "SA",
    },
    BuiltinCity {
        names: &["stockholm", "stokholm"],
        lat: 59.3293, lon: 18.0686, tz: "Europe/Stockholm",
        country_code: "SE",
    },
    BuiltinCity {
        names: &["tromso", "tromsø", "tromsoe"],
        lat: 69.6492, lon: 18.9553, tz: "Europe/Oslo",
        country_code: "NO",
    },
    BuiltinCity {
        names: &["svalbard", "longyearbyen"],
        lat: 78.2232, lon: 15.6267, tz: "Arctic/Longyearbyen",
        country_code: "NO",
    },
    BuiltinCity {
        names: &["new york", "newyork", "nyc"],
        lat: 40.7128, lon: -74.0060, tz: "America/New_York",
        country_code: "US",
    },
    BuiltinCity {
        names: &["tokyo"],
        lat: 35.6762, lon: 139.6503, tz: "Asia/Tokyo",
        country_code: "JP",
    },
    BuiltinCity {
        names: &["london"],
        lat: 51.5074, lon: -0.1278, tz: "Europe/London",
        country_code: "GB",
    },
    BuiltinCity {
        names: &["cairo", "al-qahirah"],
        lat: 30.0444, lon: 31.2357, tz: "Africa/Cairo",
        country_code: "EG",
    },
    BuiltinCity {
        names: &["istanbul"],
        lat: 41.0082, lon: 28.9784, tz: "Europe/Istanbul",
        country_code: "TR",
    },
    BuiltinCity {
        names: &["jakarta"],
        lat: -6.2088, lon: 106.8456, tz: "Asia/Jakarta",
        country_code: "ID",
    },
    BuiltinCity {
        names: &["kuala lumpur", "kl"],
        lat: 3.1390, lon: 101.6869, tz: "Asia/Kuala_Lumpur",
        country_code: "MY",
    },
    BuiltinCity {
        names: &["riyadh"],
        lat: 24.7136, lon: 46.6753, tz: "Asia/Riyadh",
        country_code: "SA",
    },
    BuiltinCity {
        names: &["dubai"],
        lat: 25.2048, lon: 55.2708, tz: "Asia/Dubai",
        country_code: "AE",
    },
    BuiltinCity {
        names: &["oslo"],
        lat: 59.9139, lon: 10.7522, tz: "Europe/Oslo",
        country_code: "NO",
    },
    BuiltinCity {
        names: &["paris"],
        lat: 48.8566, lon: 2.3522, tz: "Europe/Paris",
        country_code: "FR",
    },
    BuiltinCity {
        names: &["berlin"],
        lat: 52.5200, lon: 13.4050, tz: "Europe/Berlin",
        country_code: "DE",
    },
    BuiltinCity {
        names: &["moscow", "moskva"],
        lat: 55.7558, lon: 37.6173, tz: "Europe/Moscow",
        country_code: "RU",
    },
    BuiltinCity {
        names: &["sydney"],
        lat: -33.8688, lon: 151.2093, tz: "Australia/Sydney",
        country_code: "AU",
    },
    BuiltinCity {
        names: &["los angeles", "la"],
        lat: 34.0522, lon: -118.2437, tz: "America/Los_Angeles",
        country_code: "US",
    },
    BuiltinCity {
        names: &["dhaka", "dacca"],
        lat: 23.8103, lon: 90.4125, tz: "Asia/Dhaka",
        country_code: "BD",
    },
    BuiltinCity {
        names: &["casablanca", "dar el beida"],
        lat: 33.5731, lon: -7.5898, tz: "Africa/Casablanca",
        country_code: "MA",
    },
    BuiltinCity {
        names: &["mumbai", "bombay"],
        lat: 19.0760, lon: 72.8777, tz: "Asia/Kolkata",
        country_code: "IN",
    },
    BuiltinCity {
        names: &["delhi", "new delhi"],
        lat: 28.6139, lon: 77.2090, tz: "Asia/Kolkata",
        country_code: "IN",
    },
    BuiltinCity {
        names: &["karachi"],
        lat: 24.8607, lon: 67.0011, tz: "Asia/Karachi",
        country_code: "PK",
    },
    BuiltinCity {
        names: &["tehran"],
        lat: 35.6892, lon: 51.3890, tz: "Asia/Tehran",
        country_code: "IR",
    },
    BuiltinCity {
        names: &["baghdad"],
        lat: 33.3152, lon: 44.3661, tz: "Asia/Baghdad",
        country_code: "IQ",
    },
    BuiltinCity {
        names: &["jerusalem", "al-quds"],
        lat: 31.7683, lon: 35.2137, tz: "Asia/Jerusalem",
        country_code: "IL",
    },
    BuiltinCity {
        names: &["nairobi"],
        lat: -1.2921, lon: 36.8219, tz: "Africa/Nairobi",
        country_code: "KE",
    },
    BuiltinCity {
        names: &["lagos"],
        lat: 6.5244, lon: 3.3792, tz: "Africa/Lagos",
        country_code: "NG",
    },
];

/// Compute edit distance between two strings (Levenshtein).
fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let (m, n) = (a.len(), b.len());

    let mut prev = (0..=n).collect::<Vec<_>>();
    let mut curr = vec![0; n + 1];

    for i in 1..=m {
        curr[0] = i;
        for j in 1..=n {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            curr[j] = (prev[j] + 1)
                .min(curr[j - 1] + 1)
                .min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[n]
}

/// Search the built-in dataset with fuzzy matching.
pub fn builtin_lookup(query: &str) -> Option<ResolvedLocation> {
    builtin_lookup_with_country(query, None)
}

/// Search the built-in dataset with fuzzy matching and optional country filter.
pub fn builtin_lookup_with_country(query: &str, country: Option<&str>) -> Option<ResolvedLocation> {
    let q = query.to_lowercase();
    let country_filter = country.map(|c| c.to_uppercase());

    let candidates: Vec<&BuiltinCity> = if let Some(ref cc) = country_filter {
        BUILTIN_CITIES.iter().filter(|c| c.country_code == cc.as_str()).collect()
    } else {
        BUILTIN_CITIES.iter().collect()
    };

    // Exact match first
    for city in &candidates {
        for name in city.names {
            if *name == q {
                return Some(builtin_to_resolved(city));
            }
        }
    }

    // Substring match
    for city in &candidates {
        for name in city.names {
            if name.contains(&q) || q.contains(name) {
                return Some(builtin_to_resolved(city));
            }
        }
    }

    // Fuzzy match (edit distance <= 2)
    let mut best: Option<(&BuiltinCity, usize)> = None;
    for city in &candidates {
        for name in city.names {
            let dist = edit_distance(&q, name);
            if dist <= 2 && (best.is_none() || dist < best.unwrap().1) {
                best = Some((city, dist));
            }
        }
    }

    best.map(|(city, _)| builtin_to_resolved(city))
}

fn builtin_to_resolved(city: &BuiltinCity) -> ResolvedLocation {
    ResolvedLocation {
        name: city.names[0].to_string(),
        lat: city.lat,
        lon: city.lon,
        tz: city.tz.to_string(),
        source: LocationSource::Fallback,
        display_name: None,
        country_code: Some(city.country_code.to_string()),
        resolver_confidence: 0.95,
        disambiguated: false,
        disambiguation_note: None,
    }
}

/// A city entry for the public city list API.
#[derive(Debug, Clone, Serialize)]
pub struct CityInfo {
    pub name: String,
    pub country: String,
    pub lat: f64,
    pub lon: f64,
}

/// Return the full built-in city list (for autocomplete / API).
pub fn builtin_city_list() -> Vec<CityInfo> {
    BUILTIN_CITIES
        .iter()
        .map(|c| CityInfo {
            name: c.names[0].to_string(),
            country: c.country_code.to_string(),
            lat: c.lat,
            lon: c.lon,
        })
        .collect()
}

// ─── Nominatim provider ─────────────────────────────────────────

#[derive(Deserialize, Debug, Clone)]
pub struct NominatimResult {
    pub lat: String,
    pub lon: String,
    pub display_name: String,
    #[serde(default)]
    pub importance: Option<f64>,
    #[serde(default, rename = "type")]
    pub place_type: Option<String>,
    #[serde(default, rename = "class")]
    pub place_class: Option<String>,
    #[serde(default)]
    pub addresstype: Option<String>,
}

/// A scored Nominatim candidate for disambiguation.
#[derive(Debug, Clone)]
pub struct NominatimCandidate {
    pub name: String,
    pub display_name: String,
    pub lat: f64,
    pub lon: f64,
    pub importance: f64,
    pub place_type: String,
    pub place_class: String,
    pub country_code: String,
    pub score: f64,
}

// ─── Scoring weights ─────────────────────────────────────────────

const W_IMPORTANCE: f64 = 0.40;
const W_TYPE: f64 = 0.25;
const W_NAME: f64 = 0.20;
const W_COUNTRY: f64 = 0.15;

/// The minimum score gap between #1 and #2 to auto-disambiguate.
pub const DISAMBIGUATION_THRESHOLD: f64 = 0.10;

/// Well-known city names with their expected country codes.
/// When a single-token query matches these, we boost the expected country.
const WELL_KNOWN_CITIES: &[(&str, &str)] = &[
    ("medina", "SA"), ("madinah", "SA"),
    ("mecca", "SA"), ("makkah", "SA"),
    ("jeddah", "SA"), ("gaza", "PS"),
    ("jerusalem", "IL"), ("bethlehem", "PS"),
    ("damascus", "SY"), ("baghdad", "IQ"),
    ("cairo", "EG"), ("istanbul", "TR"),
    ("paris", "FR"), ("london", "GB"),
    ("berlin", "DE"), ("rome", "IT"),
    ("moscow", "RU"), ("tokyo", "JP"),
    ("beijing", "CN"), ("delhi", "IN"),
    ("mumbai", "IN"), ("karachi", "PK"),
    ("tehran", "IR"), ("riyadh", "SA"),
    ("dubai", "AE"), ("doha", "QA"),
    ("lima", "PE"), ("santiago", "CL"),
    ("bogota", "CO"), ("athens", "GR"),
    ("vienna", "AT"), ("lisbon", "PT"),
    ("nairobi", "KE"), ("lagos", "NG"),
    ("casablanca", "MA"), ("dhaka", "BD"),
    ("mumbai", "IN"), ("delhi", "IN"),
    ("karachi", "PK"), ("tehran", "IR"),
    ("baghdad", "IQ"), ("jerusalem", "IL"),
];

fn type_rank(place_type: &str, place_class: &str) -> f64 {
    match (place_class, place_type) {
        ("place", "city") | ("boundary", "administrative") => 1.0,
        ("place", "town") => 0.8,
        ("place", "village") => 0.4,
        ("place", "hamlet") => 0.2,
        _ => 0.5,
    }
}

fn name_similarity(query: &str, display_name: &str) -> f64 {
    let q = query.to_lowercase();
    let first_component = display_name.split(',').next().unwrap_or("").trim().to_lowercase();
    if first_component == q {
        1.0
    } else if first_component.contains(&q) || q.contains(&first_component) {
        0.8
    } else {
        let dist = edit_distance(&q, &first_component);
        if dist <= 2 { 0.6 } else { 0.3 }
    }
}

fn extract_country_code(display_name: &str) -> String {
    // Nominatim display_name ends with the country.
    // We extract it and try to map to ISO code.
    let last = display_name.split(',').next_back().unwrap_or("").trim();
    country_name_to_code(last).unwrap_or_default()
}

fn country_name_to_code(name: &str) -> Option<String> {
    let n = name.to_lowercase();
    // Common mappings — extend as needed
    let code = match n.as_str() {
        "saudi arabia" | "المملكة العربية السعودية" => "SA",
        "united states" | "united states of america" | "usa" | "us" => "US",
        "united kingdom" | "uk" | "great britain" | "england" => "GB",
        "france" => "FR", "germany" | "deutschland" => "DE",
        "italy" | "italia" => "IT", "spain" | "españa" => "ES",
        "russia" | "russian federation" => "RU",
        "china" | "people's republic of china" => "CN",
        "japan" | "日本" => "JP", "india" => "IN",
        "pakistan" => "PK", "iran" => "IR",
        "iraq" => "IQ", "turkey" | "türkiye" => "TR",
        "egypt" | "مصر" => "EG", "israel" => "IL",
        "palestine" | "palestinian territory" => "PS",
        "syria" | "syrian arab republic" => "SY",
        "jordan" => "JO", "lebanon" => "LB",
        "united arab emirates" | "uae" => "AE",
        "qatar" => "QA", "kuwait" => "KW",
        "oman" => "OM", "bahrain" => "BH",
        "yemen" => "YE",
        "nigeria" => "NG", "kenya" => "KE",
        "south africa" => "ZA", "morocco" | "maroc" => "MA",
        "ethiopia" => "ET", "tanzania" => "TZ",
        "australia" => "AU", "new zealand" | "aotearoa" => "NZ",
        "indonesia" => "ID", "malaysia" => "MY",
        "thailand" => "TH", "vietnam" | "viet nam" => "VN",
        "philippines" => "PH", "singapore" => "SG",
        "south korea" | "korea, republic of" => "KR",
        "canada" => "CA", "mexico" | "méxico" => "MX",
        "brazil" | "brasil" => "BR", "argentina" => "AR",
        "colombia" => "CO", "peru" | "perú" => "PE",
        "chile" => "CL", "sweden" | "sverige" => "SE",
        "norway" | "norge" => "NO", "denmark" | "danmark" => "DK",
        "finland" | "suomi" => "FI", "iceland" | "ísland" => "IS",
        "netherlands" | "nederland" => "NL",
        "belgium" | "belgique" | "belgië" => "BE",
        "switzerland" | "schweiz" | "suisse" => "CH",
        "austria" | "österreich" => "AT",
        "portugal" => "PT", "greece" | "ελλάδα" => "GR",
        "poland" | "polska" => "PL",
        "czech republic" | "czechia" | "česko" => "CZ",
        "hungary" | "magyarország" => "HU",
        "romania" | "românia" => "RO",
        "bangladesh" | "বাংলাদেশ" => "BD",
        "sri lanka" => "LK",
        "nepal" => "NP",
        "afghanistan" => "AF",
        "uzbekistan" => "UZ",
        "kazakhstan" => "KZ",
        "azerbaijan" => "AZ",
        "georgia" => "GE",
        _ => return None,
    };
    Some(code.to_string())
}

fn score_candidate(query: &str, candidate: &NominatimResult, country_hint: Option<&str>) -> NominatimCandidate {
    let importance = candidate.importance.unwrap_or(0.3);
    let ptype = candidate.place_type.as_deref().unwrap_or("unknown");
    let pclass = candidate.place_class.as_deref().unwrap_or("unknown");
    let country = extract_country_code(&candidate.display_name);

    let type_score = type_rank(ptype, pclass);
    let name_score = name_similarity(query, &candidate.display_name);

    // Country bonus: from explicit --country flag or from well-known list
    let q_lower = query.to_lowercase();
    let mut country_score = 0.5; // neutral
    if let Some(hint) = country_hint {
        if country == hint.to_uppercase() {
            country_score = 1.0;
        } else {
            country_score = 0.0;
        }
    } else {
        // Check well-known list
        for (known_name, expected_cc) in WELL_KNOWN_CITIES {
            if q_lower == *known_name && country == *expected_cc {
                country_score = 1.0;
                break;
            } else if q_lower == *known_name && country != *expected_cc {
                country_score = 0.1;
                break;
            }
        }
    }

    let score = W_IMPORTANCE * importance
        + W_TYPE * type_score
        + W_NAME * name_score
        + W_COUNTRY * country_score;

    let lat: f64 = candidate.lat.parse().unwrap_or(0.0);
    let lon: f64 = candidate.lon.parse().unwrap_or(0.0);
    let short_name = candidate.display_name.split(',').next().unwrap_or(query).trim().to_string();

    NominatimCandidate {
        name: short_name,
        display_name: candidate.display_name.clone(),
        lat,
        lon,
        importance,
        place_type: ptype.to_string(),
        place_class: pclass.to_string(),
        country_code: country,
        score,
    }
}

/// Resolve a city name via OpenStreetMap Nominatim, returning scored candidates.
pub fn nominatim_resolve_candidates(
    query: &str,
    country_hint: Option<&str>,
    limit: usize,
) -> Result<Vec<NominatimCandidate>, LocationError> {
    let country_param = if let Some(cc) = country_hint {
        format!("&countrycodes={}", urlencod(cc))
    } else {
        String::new()
    };

    let url = format!(
        "https://nominatim.openstreetmap.org/search?q={}&format=json&limit={}&addressdetails=0{}",
        urlencod(query),
        limit.clamp(3, 10),
        country_param,
    );

    let response = ureq::get(&url)
        .set("User-Agent", "PolarisChronos/0.5 (prayer-time-engine)")
        .call()
        .map_err(|e| LocationError::Network(e.to_string()))?;

    let results: Vec<NominatimResult> = response
        .into_json()
        .map_err(|e| LocationError::InvalidResponse(e.to_string()))?;

    if results.is_empty() {
        return Err(LocationError::NotFound(query.to_string()));
    }

    let mut candidates: Vec<NominatimCandidate> = results
        .iter()
        .map(|r| score_candidate(query, r, country_hint))
        .collect();

    candidates.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

    Ok(candidates)
}

/// Resolve a city name via OpenStreetMap Nominatim (legacy single-result).
pub fn nominatim_resolve(query: &str) -> Result<ResolvedLocation, LocationError> {
    nominatim_resolve_with_options(query, None)
}

/// Resolve with country hint.
pub fn nominatim_resolve_with_options(
    query: &str,
    country_hint: Option<&str>,
) -> Result<ResolvedLocation, LocationError> {
    let candidates = nominatim_resolve_candidates(query, country_hint, 5)?;

    if candidates.is_empty() {
        return Err(LocationError::NotFound(query.to_string()));
    }

    let top = &candidates[0];

    // Check disambiguation need
    let mut disambiguated = false;
    let mut disambiguation_note = None;

    if candidates.len() > 1 {
        let gap = top.score - candidates[1].score;

        // If the gap is too small and there's no explicit country hint, this is ambiguous
        if gap < DISAMBIGUATION_THRESHOLD && country_hint.is_none() {
            // Check if different countries
            let diff_countries = top.country_code != candidates[1].country_code;

            if diff_countries {
                // Return ambiguous error
                return Err(LocationError::Ambiguous {
                    query: query.to_string(),
                    candidates: candidates.iter().take(5).map(|c| {
                        super::types::AmbiguousCandidate {
                            name: c.display_name.clone(),
                            country: c.country_code.clone(),
                            score: c.score,
                        }
                    }).collect(),
                });
            }
        }

        // Auto-disambiguated — the top result won clearly
        if gap >= DISAMBIGUATION_THRESHOLD && top.country_code != candidates[1].country_code {
            disambiguated = true;
            disambiguation_note = Some(format!(
                "Chose {} ({}) over {} ({}) — score gap {:.2}",
                top.name, top.country_code,
                candidates[1].name, candidates[1].country_code,
                gap,
            ));
        }
    }

    // Derive timezone
    let tz = tz_from_coords(top.lat, top.lon);

    Ok(ResolvedLocation {
        name: top.name.clone(),
        lat: top.lat,
        lon: top.lon,
        tz,
        source: LocationSource::Nominatim,
        display_name: Some(top.display_name.clone()),
        country_code: if top.country_code.is_empty() { None } else { Some(top.country_code.clone()) },
        resolver_confidence: top.score.min(1.0),
        disambiguated,
        disambiguation_note,
    })
}

// ─── IP-based geolocation ───────────────────────────────────────

#[derive(Deserialize)]
struct IpApiResult {
    latitude: Option<f64>,
    longitude: Option<f64>,
    timezone: Option<String>,
    city: Option<String>,
    country_name: Option<String>,
    country_code: Option<String>,
}

/// Auto-detect location via IP geolocation.
pub fn ip_geolocate() -> Result<ResolvedLocation, LocationError> {
    let response = ureq::get("https://ipapi.co/json/")
        .set("User-Agent", "PolarisChronos/0.5")
        .call()
        .map_err(|e| LocationError::Network(e.to_string()))?;

    let r: IpApiResult = response
        .into_json()
        .map_err(|e| LocationError::InvalidResponse(e.to_string()))?;

    let lat = r.latitude.ok_or_else(|| LocationError::InvalidResponse("no latitude".into()))?;
    let lon = r.longitude.ok_or_else(|| LocationError::InvalidResponse("no longitude".into()))?;
    let tz = r.timezone.unwrap_or_else(|| tz_from_coords(lat, lon));
    let city = r.city.unwrap_or_else(|| "Unknown".into());
    let country = r.country_name.unwrap_or_default();
    let cc = r.country_code;

    let name = if country.is_empty() {
        city
    } else {
        format!("{}, {}", city, country)
    };

    Ok(ResolvedLocation {
        name,
        lat,
        lon,
        tz,
        source: LocationSource::IpApi,
        display_name: None,
        country_code: cc,
        resolver_confidence: 0.8,
        disambiguated: false,
        disambiguation_note: None,
    })
}

// ─── Timezone estimation from coordinates ───────────────────────

/// Approximate IANA timezone from longitude (rough but works offline).
/// This is a fallback — Nominatim results get a better estimate.
fn tz_from_coords(lat: f64, lon: f64) -> String {
    // Try the timezone API first (fast, free, no key)
    if let Ok(tz) = tz_from_api(lat, lon) {
        return tz;
    }

    // Fallback: rough longitude-based estimation
    let offset_hours = (lon / 15.0).round() as i32;
    // Map to common IANA zones by rough offset
    match offset_hours {
        -12..=-10 => "Pacific/Honolulu".into(),
        -9 => "America/Anchorage".into(),
        -8 => "America/Los_Angeles".into(),
        -7 => "America/Denver".into(),
        -6 => "America/Chicago".into(),
        -5 => "America/New_York".into(),
        -4 => "America/Halifax".into(),
        -3 => "America/Sao_Paulo".into(),
        -2..=-1 => "Atlantic/Azores".into(),
        0 => "Europe/London".into(),
        1 => "Europe/Paris".into(),
        2 => "Europe/Helsinki".into(),
        3 => "Europe/Moscow".into(),
        4 => "Asia/Dubai".into(),
        5 => "Asia/Karachi".into(),
        6 => "Asia/Dhaka".into(),
        7 => "Asia/Bangkok".into(),
        8 => "Asia/Shanghai".into(),
        9 => "Asia/Tokyo".into(),
        10 => "Australia/Sydney".into(),
        11 => "Pacific/Noumea".into(),
        12 => "Pacific/Auckland".into(),
        _ => "UTC".into(),
    }
}

fn tz_from_api(lat: f64, lon: f64) -> Result<String, LocationError> {
    let url = format!(
        "https://www.timeapi.io/api/timezone/coordinate?latitude={}&longitude={}",
        lat, lon
    );

    let response = ureq::get(&url)
        .set("User-Agent", "PolarisChronos/0.5")
        .timeout(std::time::Duration::from_secs(3))
        .call()
        .map_err(|e| LocationError::Network(e.to_string()))?;

    let val: serde_json::Value = response
        .into_json()
        .map_err(|e| LocationError::InvalidResponse(e.to_string()))?;

    val.get("timeZone")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| LocationError::InvalidResponse("no timeZone field".into()))
}

// ─── URL encoding (minimal, no extra dep) ───────────────────────

fn urlencod(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            ' ' => "%20".to_string(),
            '&' => "%26".to_string(),
            '=' => "%3D".to_string(),
            '+' => "%2B".to_string(),
            ',' => "%2C".to_string(),
            _ if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '~' => {
                c.to_string()
            }
            _ => format!("%{:02X}", c as u32),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_exact() {
        let loc = builtin_lookup("Mecca").unwrap();
        assert_eq!(loc.name, "mecca");
        assert!((loc.lat - 21.4225).abs() < 0.01);
        assert_eq!(loc.tz, "Asia/Riyadh");
        assert_eq!(loc.source, LocationSource::Fallback);
        assert_eq!(loc.country_code, Some("SA".to_string()));
    }

    #[test]
    fn test_builtin_case_insensitive() {
        let loc = builtin_lookup("STOCKHOLM").unwrap();
        assert_eq!(loc.name, "stockholm");
    }

    #[test]
    fn test_builtin_alias() {
        let loc = builtin_lookup("mekka").unwrap();
        assert_eq!(loc.name, "mecca");
    }

    #[test]
    fn test_builtin_fuzzy() {
        // "stokholm" → "stockholm" (edit distance 1)
        let loc = builtin_lookup("stokholm").unwrap();
        assert_eq!(loc.name, "stockholm");
    }

    #[test]
    fn test_builtin_multi_word() {
        let loc = builtin_lookup("new york").unwrap();
        assert_eq!(loc.name, "new york");
        assert_eq!(loc.tz, "America/New_York");
    }

    #[test]
    fn test_builtin_alias_nyc() {
        let loc = builtin_lookup("NYC").unwrap();
        assert_eq!(loc.name, "new york");
    }

    #[test]
    fn test_builtin_not_found() {
        assert!(builtin_lookup("xyznonexistent").is_none());
    }

    #[test]
    fn test_builtin_with_country_filter() {
        let loc = builtin_lookup_with_country("medina", Some("SA")).unwrap();
        assert_eq!(loc.country_code, Some("SA".to_string()));
        assert_eq!(loc.tz, "Asia/Riyadh");
    }

    #[test]
    fn test_edit_distance() {
        assert_eq!(edit_distance("kitten", "sitting"), 3);
        assert_eq!(edit_distance("stockholm", "stokholm"), 1);
        assert_eq!(edit_distance("mecca", "mekka"), 2);
        assert_eq!(edit_distance("abc", "abc"), 0);
    }

    #[test]
    fn test_country_name_to_code() {
        assert_eq!(country_name_to_code("Saudi Arabia"), Some("SA".to_string()));
        assert_eq!(country_name_to_code("France"), Some("FR".to_string()));
        assert_eq!(country_name_to_code("Unknown Land"), None);
    }

    #[test]
    fn test_type_rank() {
        assert!(type_rank("city", "place") > type_rank("village", "place"));
        assert!(type_rank("town", "place") > type_rank("hamlet", "place"));
    }

    #[test]
    fn test_name_similarity() {
        assert_eq!(name_similarity("paris", "Paris, Île-de-France, France"), 1.0);
        assert!(name_similarity("paris", "Paris, TX, US") > 0.5);
    }
}
