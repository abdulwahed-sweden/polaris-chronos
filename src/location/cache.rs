//! File-based location cache at ~/.polaris/cache.json.
//!
//! TTL: 30 days. Case-insensitive keys.
//! Schema v2: adds display_name, country_code, source_name, confidence.
//! Backward compatible: missing fields default gracefully.

use super::types::{LocationSource, ResolvedLocation};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

const CACHE_TTL_MS: i64 = 30 * 24 * 3600 * 1000; // 30 days in ms

#[derive(Serialize, Deserialize, Clone)]
struct CacheEntry {
    lat: f64,
    lon: f64,
    tz: String,
    name: String,
    timestamp: i64,
    // v2 fields (backward compatible via serde defaults)
    #[serde(default)]
    display_name: Option<String>,
    #[serde(default)]
    country_code: Option<String>,
    #[serde(default)]
    source_name: Option<String>,
    #[serde(default = "default_confidence")]
    confidence: f64,
}

fn default_confidence() -> f64 {
    1.0
}

/// The location cache.
pub struct LocationCache {
    path: PathBuf,
    entries: HashMap<String, CacheEntry>,
}

impl LocationCache {
    /// Load cache from the default location (~/.polaris/cache.json).
    pub fn load() -> Self {
        let path = Self::default_path();
        let entries = Self::read_file(&path).unwrap_or_default();
        Self { path, entries }
    }

    /// Load cache from a specific path (for testing).
    pub fn load_from(path: PathBuf) -> Self {
        let entries = Self::read_file(&path).unwrap_or_default();
        Self { path, entries }
    }

    fn default_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".polaris")
            .join("cache.json")
    }

    fn read_file(path: &PathBuf) -> Option<HashMap<String, CacheEntry>> {
        let data = fs::read_to_string(path).ok()?;
        serde_json::from_str(&data).ok()
    }

    /// Look up a city in the cache. Returns None if missing or expired.
    pub fn get(&self, query: &str) -> Option<ResolvedLocation> {
        let key = query.to_lowercase();
        let entry = self.entries.get(&key)?;

        let now = chrono::Utc::now().timestamp_millis();
        if now - entry.timestamp > CACHE_TTL_MS {
            return None; // expired
        }

        Some(ResolvedLocation {
            name: entry.name.clone(),
            lat: entry.lat,
            lon: entry.lon,
            tz: entry.tz.clone(),
            source: LocationSource::Cache,
            display_name: entry.display_name.clone(),
            country_code: entry.country_code.clone(),
            resolver_confidence: entry.confidence,
            disambiguated: false,
            disambiguation_note: None,
        })
    }

    /// Get the most recently cached location (for --auto fallback).
    pub fn most_recent(&self) -> Option<ResolvedLocation> {
        let now = chrono::Utc::now().timestamp_millis();
        self.entries
            .values()
            .filter(|e| now - e.timestamp <= CACHE_TTL_MS)
            .max_by_key(|e| e.timestamp)
            .map(|e| ResolvedLocation {
                name: e.name.clone(),
                lat: e.lat,
                lon: e.lon,
                tz: e.tz.clone(),
                source: LocationSource::Cache,
                display_name: e.display_name.clone(),
                country_code: e.country_code.clone(),
                resolver_confidence: e.confidence,
                disambiguated: false,
                disambiguation_note: None,
            })
    }

    /// Store a resolved location in the cache and persist to disk.
    /// Caches under both the resolved name AND the original query (if different).
    pub fn put(&mut self, resolved: &ResolvedLocation) {
        let key = resolved.name.to_lowercase();
        let entry = CacheEntry {
            lat: resolved.lat,
            lon: resolved.lon,
            tz: resolved.tz.clone(),
            name: resolved.name.clone(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            display_name: resolved.display_name.clone(),
            country_code: resolved.country_code.clone(),
            source_name: Some(resolved.source.to_string()),
            confidence: resolved.resolver_confidence,
        };
        self.entries.insert(key, entry);
        self.persist();
    }

    /// Store under a specific query key (for caching original query → resolved result).
    pub fn put_with_key(&mut self, query: &str, resolved: &ResolvedLocation) {
        let key = query.to_lowercase();
        let entry = CacheEntry {
            lat: resolved.lat,
            lon: resolved.lon,
            tz: resolved.tz.clone(),
            name: resolved.name.clone(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            display_name: resolved.display_name.clone(),
            country_code: resolved.country_code.clone(),
            source_name: Some(resolved.source.to_string()),
            confidence: resolved.resolver_confidence,
        };
        self.entries.insert(key, entry);
        // Also cache under the resolved name
        let name_key = resolved.name.to_lowercase();
        if name_key != query.to_lowercase() {
            self.entries.insert(name_key, CacheEntry {
                lat: resolved.lat,
                lon: resolved.lon,
                tz: resolved.tz.clone(),
                name: resolved.name.clone(),
                timestamp: chrono::Utc::now().timestamp_millis(),
                display_name: resolved.display_name.clone(),
                country_code: resolved.country_code.clone(),
                source_name: Some(resolved.source.to_string()),
                confidence: resolved.resolver_confidence,
            });
        }
        self.persist();
    }

    fn persist(&self) {
        if let Some(parent) = self.path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(&self.entries) {
            let _ = fs::write(&self.path, json);
        }
    }

    /// Number of entries (for testing).
    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_cache() -> (LocationCache, TempDir) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("cache.json");
        (LocationCache::load_from(path), dir)
    }

    #[test]
    fn test_cache_put_get() {
        let (mut cache, _dir) = test_cache();

        let loc = ResolvedLocation {
            name: "Stockholm".into(),
            lat: 59.3293,
            lon: 18.0686,
            tz: "Europe/Stockholm".into(),
            source: LocationSource::Nominatim,
            display_name: Some("Stockholm, Sweden".into()),
            country_code: Some("SE".into()),
            resolver_confidence: 0.92,
            disambiguated: false,
            disambiguation_note: None,
        };
        cache.put(&loc);

        let result = cache.get("stockholm").unwrap();
        assert_eq!(result.name, "Stockholm");
        assert_eq!(result.source, LocationSource::Cache);
        assert!((result.lat - 59.3293).abs() < 0.001);
        assert_eq!(result.country_code, Some("SE".to_string()));
        assert!((result.resolver_confidence - 0.92).abs() < 0.01);
    }

    #[test]
    fn test_cache_case_insensitive() {
        let (mut cache, _dir) = test_cache();

        let loc = ResolvedLocation {
            name: "New York".into(),
            lat: 40.7128,
            lon: -74.006,
            tz: "America/New_York".into(),
            source: LocationSource::Nominatim,
            display_name: None,
            country_code: Some("US".into()),
            resolver_confidence: 0.95,
            disambiguated: false,
            disambiguation_note: None,
        };
        cache.put(&loc);

        assert!(cache.get("NEW YORK").is_some());
        assert!(cache.get("new york").is_some());
    }

    #[test]
    fn test_cache_miss() {
        let (cache, _dir) = test_cache();
        assert!(cache.get("nonexistent").is_none());
    }

    #[test]
    fn test_cache_persistence() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("cache.json");

        // Write
        {
            let mut cache = LocationCache::load_from(path.clone());
            cache.put(&ResolvedLocation {
                name: "Tokyo".into(),
                lat: 35.6762,
                lon: 139.6503,
                tz: "Asia/Tokyo".into(),
                source: LocationSource::Nominatim,
                display_name: None,
                country_code: Some("JP".into()),
                resolver_confidence: 0.9,
                disambiguated: false,
                disambiguation_note: None,
            });
        }

        // Read back
        let cache2 = LocationCache::load_from(path);
        let result = cache2.get("tokyo").unwrap();
        assert_eq!(result.name, "Tokyo");
    }

    #[test]
    fn test_most_recent() {
        let (mut cache, _dir) = test_cache();

        cache.put(&ResolvedLocation {
            name: "First".into(),
            lat: 0.0, lon: 0.0,
            tz: "UTC".into(),
            source: LocationSource::Nominatim,
            display_name: None,
            country_code: None,
            resolver_confidence: 0.5,
            disambiguated: false,
            disambiguation_note: None,
        });
        std::thread::sleep(std::time::Duration::from_millis(10));
        cache.put(&ResolvedLocation {
            name: "Second".into(),
            lat: 1.0, lon: 1.0,
            tz: "UTC".into(),
            source: LocationSource::Nominatim,
            display_name: None,
            country_code: None,
            resolver_confidence: 0.8,
            disambiguated: false,
            disambiguation_note: None,
        });

        let recent = cache.most_recent().unwrap();
        assert_eq!(recent.name, "Second");
    }

    #[test]
    fn test_cache_backward_compatible() {
        // Simulate a v1 cache entry (no new fields)
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("cache.json");
        let v1_json = r#"{
            "stockholm": {
                "lat": 59.3293,
                "lon": 18.0686,
                "tz": "Europe/Stockholm",
                "name": "Stockholm",
                "timestamp": 9999999999999
            }
        }"#;
        fs::write(&path, v1_json).unwrap();

        let cache = LocationCache::load_from(path);
        let result = cache.get("stockholm").unwrap();
        assert_eq!(result.name, "Stockholm");
        assert!(result.country_code.is_none());
        assert!((result.resolver_confidence - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_put_with_key() {
        let (mut cache, _dir) = test_cache();

        let loc = ResolvedLocation {
            name: "Al Madinah Al Munawwarah".into(),
            lat: 24.47,
            lon: 39.61,
            tz: "Asia/Riyadh".into(),
            source: LocationSource::Nominatim,
            display_name: Some("Medina, Saudi Arabia".into()),
            country_code: Some("SA".into()),
            resolver_confidence: 0.9,
            disambiguated: false,
            disambiguation_note: None,
        };
        cache.put_with_key("medina", &loc);

        // Should be accessible via both keys
        assert!(cache.get("medina").is_some());
        assert!(cache.get("al madinah al munawwarah").is_some());
    }
}
