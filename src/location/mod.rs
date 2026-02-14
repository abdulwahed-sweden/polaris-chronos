//! Location intelligence subsystem for Polaris Chronos.
//!
//! Provides city name resolution, IP-based geolocation, local caching,
//! and a built-in fallback dataset with fuzzy matching.

pub mod cache;
pub mod providers;
pub mod resolver;
pub mod types;

pub use resolver::LocationResolver;
pub use types::{LocationError, LocationSource, ResolvedLocation, ResolveOptions};
