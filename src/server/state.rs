use crate::location::LocationResolver;
use crate::solver::SolverOutput;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

/// Cache entry with TTL tracking.
struct CacheEntry {
    output: SolverOutput,
    created: Instant,
}

/// In-memory computation cache with TTL eviction.
pub struct ComputeCache {
    entries: HashMap<String, CacheEntry>,
    ttl_secs: u64,
}

impl ComputeCache {
    pub fn new(ttl_secs: u64) -> Self {
        Self {
            entries: HashMap::new(),
            ttl_secs,
        }
    }

    /// Build a cache key from computation parameters.
    pub fn key(lat: f64, lon: f64, date: &str, strategy: &str) -> String {
        format!("{:.4},{:.4},{},{}", lat, lon, date, strategy)
    }

    /// Get a cached result if it exists and hasn't expired.
    pub fn get(&mut self, key: &str) -> Option<SolverOutput> {
        if let Some(entry) = self.entries.get(key) {
            if entry.created.elapsed().as_secs() < self.ttl_secs {
                return Some(entry.output.clone());
            }
            // Expired — remove it
            self.entries.remove(key);
        }
        None
    }

    /// Store a computation result.
    pub fn put(&mut self, key: String, output: SolverOutput) {
        // Evict old entries if cache gets too large
        if self.entries.len() > 1000 {
            let cutoff = Instant::now();
            self.entries.retain(|_, v| cutoff.duration_since(v.created).as_secs() < self.ttl_secs);
        }
        self.entries.insert(key, CacheEntry {
            output,
            created: Instant::now(),
        });
    }
}

pub struct AppState {
    pub resolver: Mutex<LocationResolver>,
    pub cache: Mutex<ComputeCache>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            resolver: Mutex::new(LocationResolver::new()),
            cache: Mutex::new(ComputeCache::new(6 * 3600)), // 6 hour TTL
        }
    }
}
