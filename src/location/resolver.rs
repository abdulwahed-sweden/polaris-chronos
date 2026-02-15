//! Location resolver — orchestrates the fallback chain.
//!
//! City flow:  Cache → Nominatim (with disambiguation) → simplified query → built-in dataset → error
//! Auto flow:  IP API → last cached location → error

use super::cache::LocationCache;
use super::providers;
use super::types::{LocationError, LocationSource, ResolvedLocation, ResolveOptions};

/// The location resolver with its fallback pipeline.
pub struct LocationResolver {
    cache: LocationCache,
    offline: bool,
}

impl LocationResolver {
    pub fn new() -> Self {
        Self {
            cache: LocationCache::load(),
            offline: false,
        }
    }

    /// Create a resolver with a specific cache (for testing).
    pub fn with_cache(cache: LocationCache) -> Self {
        Self { cache, offline: false }
    }

    /// Set offline mode — skip network calls.
    pub fn set_offline(&mut self, offline: bool) {
        self.offline = offline;
    }

    /// Resolve a city name through the full fallback chain (no options).
    pub fn resolve_city(&mut self, query: &str) -> Result<ResolvedLocation, LocationError> {
        self.resolve_city_with_opts(query, &ResolveOptions::default())
    }

    /// Resolve a city name with options (country hint, topk debug).
    pub fn resolve_city_with_opts(
        &mut self,
        query: &str,
        opts: &ResolveOptions,
    ) -> Result<ResolvedLocation, LocationError> {
        // Parse comma-separated queries: "Medina, Saudi Arabia" → city="Medina", country_hint="SA"
        let (city_query, parsed_country) = parse_query_with_hint(query);
        let country_hint = opts.country.as_deref().or(parsed_country.as_deref());

        // 1. Check cache (skip if country filter is active and cache might be stale)
        if country_hint.is_none() {
            if let Some(loc) = self.cache.get(&city_query) {
                return Ok(loc);
            }
        }

        // 2. Try Nominatim with disambiguation (if online)
        if !self.offline {
            // If --topk is set, show candidates and proceed
            if let Some(topk) = opts.topk {
                match providers::nominatim_resolve_candidates(&city_query, country_hint, topk) {
                    Ok(candidates) => {
                        eprintln!("  Top-{} candidates for '{}':", topk, query);
                        for (i, c) in candidates.iter().enumerate().take(topk) {
                            eprintln!(
                                "    {}. {} [{}] score={:.3} (importance={:.3}, type={}/{})",
                                i + 1, c.display_name, c.country_code,
                                c.score, c.importance, c.place_class, c.place_type,
                            );
                        }
                    }
                    Err(e) => eprintln!("  Warning: --topk failed: {}", e),
                }
            }

            match providers::nominatim_resolve_with_options(&city_query, country_hint) {
                Ok(loc) => {
                    self.cache.put_with_key(query, &loc);
                    return Ok(loc);
                }
                Err(LocationError::Ambiguous { .. }) => {
                    // Before propagating ambiguity, check if built-in has a confident match.
                    // This handles cases like "Medina" where Nominatim doesn't return the
                    // well-known Saudi city but our built-in dataset knows it.
                    if let Some(mut builtin) = providers::builtin_lookup_with_country(&city_query, country_hint) {
                        builtin.disambiguated = true;
                        builtin.disambiguation_note = Some(format!(
                            "Nominatim returned ambiguous results; used built-in dataset for {} ({})",
                            builtin.name,
                            builtin.country_code.as_deref().unwrap_or("??"),
                        ));
                        self.cache.put_with_key(query, &builtin);
                        return Ok(builtin);
                    }
                    // No built-in match — propagate ambiguity
                    return Err(LocationError::Ambiguous {
                        query: query.to_string(),
                        candidates: match providers::nominatim_resolve_candidates(&city_query, None, 5) {
                            Ok(c) => c.iter().take(5).map(|c| super::types::AmbiguousCandidate {
                                name: c.display_name.clone(),
                                country: c.country_code.clone(),
                                country_name: providers::country_display_name(&c.country_code).to_string(),
                                lat: c.lat,
                                lon: c.lon,
                                tz: providers::tz_from_coords(c.lat, c.lon),
                                score: c.score,
                            }).collect(),
                            Err(_) => vec![],
                        },
                    });
                }
                Err(_) => {} // fall through to next attempt
            }

            // 3. Try simplified query (remove special chars, lowercase)
            let simplified = simplify_query(&city_query);
            if simplified != city_query.to_lowercase() {
                match providers::nominatim_resolve_with_options(&simplified, country_hint) {
                    Ok(loc) => {
                        self.cache.put_with_key(query, &loc);
                        return Ok(loc);
                    }
                    Err(_) => {}
                }
            }
        }

        // 4. Try built-in dataset (always available)
        if let Some(loc) = providers::builtin_lookup_with_country(&city_query, country_hint) {
            return Ok(loc);
        }

        Err(LocationError::NotFound(query.to_string()))
    }

    /// Auto-detect location via IP.
    pub fn resolve_auto(&mut self) -> Result<ResolvedLocation, LocationError> {
        // 1. Try IP API
        if !self.offline {
            match providers::ip_geolocate() {
                Ok(loc) => {
                    self.cache.put(&loc);
                    return Ok(loc);
                }
                Err(_) => {}
            }
        }

        // 2. Fallback to most recent cached location
        if let Some(loc) = self.cache.most_recent() {
            return Ok(loc);
        }

        Err(LocationError::Network(
            "Could not auto-detect location. Try --city instead.".into()
        ))
    }

    /// Create a ResolvedLocation from manual lat/lon input.
    pub fn from_manual(lat: f64, lon: f64, tz_override: Option<&str>) -> ResolvedLocation {
        let tz = tz_override
            .map(|s| s.to_string())
            .unwrap_or_else(|| "UTC".into());

        ResolvedLocation {
            name: format!("{:.4}, {:.4}", lat, lon),
            lat,
            lon,
            tz,
            source: LocationSource::Manual,
            display_name: None,
            country_code: None,
            resolver_confidence: 1.0,
            disambiguated: false,
            disambiguation_note: None,
        }
    }
}

/// Parse "Medina, Saudi Arabia" → ("Medina", Some("SA"))
/// Parse "Medina" → ("Medina", None)
fn parse_query_with_hint(query: &str) -> (String, Option<String>) {
    let parts: Vec<&str> = query.splitn(2, ',').collect();
    if parts.len() == 2 {
        let city = parts[0].trim().to_string();
        let hint_raw = parts[1].trim();
        // Try to parse as country code (2 letters) or country name
        if hint_raw.len() == 2 && hint_raw.chars().all(|c| c.is_ascii_alphabetic()) {
            return (city, Some(hint_raw.to_uppercase()));
        }
        // Try to map country name to code
        if let Some(code) = country_name_to_hint(hint_raw) {
            return (city, Some(code));
        }
        // Not a recognized country — pass the full query as-is (might be a multi-part city name)
        return (query.to_string(), None);
    }
    (query.to_string(), None)
}

/// Map common country names/partial names to ISO codes for hints.
fn country_name_to_hint(name: &str) -> Option<String> {
    let n = name.to_lowercase();
    let code = match n.as_str() {
        "saudi arabia" | "saudi" | "ksa" => "SA",
        "united states" | "usa" | "us" | "america" => "US",
        "united kingdom" | "uk" | "britain" | "england" => "GB",
        "france" | "french" => "FR",
        "germany" => "DE",
        "italy" | "italia" => "IT",
        "spain" | "españa" => "ES",
        "russia" => "RU",
        "china" => "CN",
        "japan" => "JP",
        "india" => "IN",
        "pakistan" => "PK",
        "iran" => "IR",
        "iraq" => "IQ",
        "turkey" | "türkiye" => "TR",
        "egypt" => "EG",
        "israel" => "IL",
        "palestine" => "PS",
        "syria" => "SY",
        "jordan" => "JO",
        "lebanon" => "LB",
        "uae" | "emirates" => "AE",
        "qatar" => "QA",
        "kuwait" => "KW",
        "oman" => "OM",
        "yemen" => "YE",
        "nigeria" => "NG",
        "kenya" => "KE",
        "south africa" => "ZA",
        "morocco" => "MA",
        "australia" => "AU",
        "new zealand" => "NZ",
        "indonesia" => "ID",
        "malaysia" => "MY",
        "canada" => "CA",
        "mexico" => "MX",
        "brazil" | "brasil" => "BR",
        "argentina" => "AR",
        "colombia" => "CO",
        "peru" => "PE",
        "chile" => "CL",
        "sweden" | "sverige" => "SE",
        "norway" | "norge" => "NO",
        "denmark" => "DK",
        "finland" => "FI",
        "iceland" => "IS",
        "netherlands" => "NL",
        "belgium" => "BE",
        "switzerland" => "CH",
        "austria" => "AT",
        "portugal" => "PT",
        "greece" => "GR",
        "poland" => "PL",
        _ => return None,
    };
    Some(code.to_string())
}

/// Simplify a query for retry: lowercase, strip accents/diacritics, collapse spaces.
fn simplify_query(q: &str) -> String {
    q.to_lowercase()
        .replace('ø', "o")
        .replace('å', "a")
        .replace('ä', "a")
        .replace('ö', "o")
        .replace('ü', "u")
        .replace('ß', "ss")
        .replace('é', "e")
        .replace('è', "e")
        .replace('ê', "e")
        .replace('ñ', "n")
        .replace('ã', "a")
        .replace('õ', "o")
        .replace('ç', "c")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::cache::LocationCache;
    use tempfile::TempDir;

    fn offline_resolver() -> (LocationResolver, TempDir) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("cache.json");
        let cache = LocationCache::load_from(path);
        let mut resolver = LocationResolver::with_cache(cache);
        resolver.set_offline(true);
        (resolver, dir)
    }

    #[test]
    fn test_resolve_builtin_fallback() {
        let (mut resolver, _dir) = offline_resolver();
        let loc = resolver.resolve_city("Mecca").unwrap();
        assert_eq!(loc.source, LocationSource::Fallback);
        assert!((loc.lat - 21.4225).abs() < 0.01);
    }

    #[test]
    fn test_resolve_cache_hit() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("cache.json");
        let mut cache = LocationCache::load_from(path);
        cache.put(&ResolvedLocation {
            name: "TestCity".into(),
            lat: 10.0,
            lon: 20.0,
            tz: "UTC".into(),
            source: LocationSource::Nominatim,
            display_name: None,
            country_code: None,
            resolver_confidence: 0.9,
            disambiguated: false,
            disambiguation_note: None,
        });

        let mut resolver = LocationResolver::with_cache(cache);
        resolver.set_offline(true);

        let loc = resolver.resolve_city("testcity").unwrap();
        assert_eq!(loc.source, LocationSource::Cache);
        assert_eq!(loc.name, "TestCity");
    }

    #[test]
    fn test_resolve_not_found() {
        let (mut resolver, _dir) = offline_resolver();
        let result = resolver.resolve_city("xyznonexistentcity123");
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_fuzzy_builtin() {
        let (mut resolver, _dir) = offline_resolver();
        // "stokholm" → "stockholm" via fuzzy match
        let loc = resolver.resolve_city("stokholm").unwrap();
        assert_eq!(loc.name, "stockholm");
    }

    #[test]
    fn test_resolve_alias() {
        let (mut resolver, _dir) = offline_resolver();
        let loc = resolver.resolve_city("NYC").unwrap();
        assert_eq!(loc.name, "new york");
    }

    #[test]
    fn test_simplify_query() {
        assert_eq!(simplify_query("Tromsø"), "tromso");
        assert_eq!(simplify_query("São Paulo"), "sao paulo");
        assert_eq!(simplify_query("  Multiple   Spaces  "), "multiple spaces");
    }

    #[test]
    fn test_manual_location() {
        let loc = LocationResolver::from_manual(59.33, 18.07, Some("Europe/Stockholm"));
        assert_eq!(loc.source, LocationSource::Manual);
        assert_eq!(loc.tz, "Europe/Stockholm");
    }

    #[test]
    fn test_auto_offline_with_cache() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("cache.json");
        let mut cache = LocationCache::load_from(path);
        cache.put(&ResolvedLocation {
            name: "LastKnown".into(),
            lat: 50.0,
            lon: 10.0,
            tz: "Europe/Berlin".into(),
            source: LocationSource::IpApi,
            display_name: None,
            country_code: Some("DE".into()),
            resolver_confidence: 0.8,
            disambiguated: false,
            disambiguation_note: None,
        });

        let mut resolver = LocationResolver::with_cache(cache);
        resolver.set_offline(true);

        let loc = resolver.resolve_auto().unwrap();
        assert_eq!(loc.name, "LastKnown");
        assert_eq!(loc.source, LocationSource::Cache);
    }

    #[test]
    fn test_auto_offline_no_cache() {
        let (mut resolver, _dir) = offline_resolver();
        assert!(resolver.resolve_auto().is_err());
    }

    #[test]
    fn test_parse_query_comma_country() {
        let (city, cc) = parse_query_with_hint("Medina, Saudi Arabia");
        assert_eq!(city, "Medina");
        assert_eq!(cc, Some("SA".to_string()));
    }

    #[test]
    fn test_parse_query_comma_iso_code() {
        let (city, cc) = parse_query_with_hint("Medina, SA");
        assert_eq!(city, "Medina");
        assert_eq!(cc, Some("SA".to_string()));
    }

    #[test]
    fn test_parse_query_no_comma() {
        let (city, cc) = parse_query_with_hint("Stockholm");
        assert_eq!(city, "Stockholm");
        assert_eq!(cc, None);
    }

    #[test]
    fn test_resolve_medina_builtin_with_country() {
        let (mut resolver, _dir) = offline_resolver();
        let opts = ResolveOptions { country: Some("SA".to_string()), topk: None };
        let loc = resolver.resolve_city_with_opts("Medina", &opts).unwrap();
        assert_eq!(loc.country_code, Some("SA".to_string()));
        assert_eq!(loc.tz, "Asia/Riyadh");
    }

    #[test]
    fn test_resolve_comma_medina_saudi_builtin() {
        let (mut resolver, _dir) = offline_resolver();
        let loc = resolver.resolve_city("Medina, Saudi Arabia").unwrap();
        assert_eq!(loc.country_code, Some("SA".to_string()));
        assert!((loc.lat - 24.4686).abs() < 0.01);
    }

    #[test]
    fn test_resolve_gaza_offline() {
        let (mut resolver, _dir) = offline_resolver();
        let loc = resolver.resolve_city("Gaza").unwrap();
        assert_eq!(loc.name, "gaza");
        assert_eq!(loc.country_code, Some("PS".to_string()));
        assert_eq!(loc.tz, "Asia/Gaza");
    }

    #[test]
    fn test_resolve_jerusalem_offline() {
        let (mut resolver, _dir) = offline_resolver();
        let loc = resolver.resolve_city("Jerusalem").unwrap();
        assert_eq!(loc.name, "jerusalem");
        assert_eq!(loc.country_code, Some("PS".to_string()));
    }
}
