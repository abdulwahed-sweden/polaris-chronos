//! Location intelligence subsystem for Polaris Chronos.
//!
//! Provides city name resolution, IP-based geolocation, local caching,
//! and a built-in fallback dataset with fuzzy matching.

pub mod cache;
pub mod providers;
pub mod resolver;
pub mod types;

pub use providers::{builtin_city_list, CityInfo, country_display_name, country_display_name_ar, format_coords};
pub use resolver::LocationResolver;
pub use types::{LocationError, LocationSource, ResolvedLocation, ResolveOptions};
