use chrono::{NaiveDate, Utc};
use chrono_tz::Tz;
use clap::Parser;
use polaris_chronos::location::{LocationResolver, ResolvedLocation, ResolveOptions};
use polaris_chronos::schedule::GapStrategy;
use polaris_chronos::solver::{Solver, render_ascii_timeline};

/// Polaris Chronos v0.5 — Adaptive Compensation Prayer Time Engine
///
/// Computes prayer times for any location on Earth, including polar regions.
/// Supports city names, auto-detection, and raw coordinates.
///
/// Examples:
///   polaris Stockholm
///   polaris --city "New York" --date 2026-03-20
///   polaris --city "Medina, Saudi Arabia"
///   polaris --city Medina --country SA
///   polaris --auto --now
///   polaris --lat 21.4225 --lon 39.8262 --tz Asia/Riyadh
#[derive(Parser)]
#[command(name = "polaris", version, about, long_about = None)]
struct Cli {
    /// City name (positional). Example: polaris Stockholm
    #[arg(index = 1)]
    city_positional: Option<String>,

    /// City name (named). Example: --city "New York"
    #[arg(long)]
    city: Option<String>,

    /// Auto-detect location via IP geolocation.
    #[arg(long, short = 'a')]
    auto: bool,

    /// Latitude (-90 to 90). Legacy mode.
    #[arg(long, allow_hyphen_values = true)]
    lat: Option<f64>,

    /// Longitude (-180 to 180). Legacy mode.
    #[arg(long, allow_hyphen_values = true)]
    lon: Option<f64>,

    /// Date (YYYY-MM-DD). Defaults to today.
    #[arg(long, short = 'd')]
    date: Option<String>,

    /// IANA timezone override (e.g. Europe/Oslo).
    #[arg(long)]
    tz: Option<String>,

    /// Show current prayer and time to next.
    #[arg(long)]
    now: bool,

    /// Output the sampled altitude wave.
    #[arg(long)]
    debug_wave: bool,

    /// Offline mode: only use cache and built-in data.
    #[arg(long)]
    offline: bool,

    /// Gap strategy for polar states: "strict" (None for missing events)
    /// or "projected45" (project from adaptive reference latitude).
    #[arg(long, default_value = "projected45", value_parser = parse_strategy)]
    strategy: GapStrategy,

    /// Show confidence scores in the ASCII timeline.
    #[arg(long)]
    show_confidence: bool,

    /// Country hint (ISO 3166-1 alpha-2, e.g. SA, US, FR).
    /// Filters Nominatim results to this country.
    #[arg(long)]
    country: Option<String>,

    /// Debug: show top-K candidates from Nominatim.
    #[arg(long)]
    topk: Option<usize>,
}

fn parse_strategy(s: &str) -> Result<GapStrategy, String> {
    match s.to_lowercase().as_str() {
        "strict" => Ok(GapStrategy::Strict),
        "projected45" | "projected" => Ok(GapStrategy::Projected45),
        _ => Err(format!("Unknown strategy '{}'. Use 'strict' or 'projected45'.", s)),
    }
}

fn main() {
    let cli = Cli::parse();

    // ── Resolve location ────────────────────────────────────────

    let mut resolver = LocationResolver::new();
    if cli.offline {
        resolver.set_offline(true);
    }

    let opts = ResolveOptions {
        country: cli.country.clone(),
        topk: cli.topk,
    };

    let resolved = resolve_location(&cli, &mut resolver, &opts);

    // ── Parse date ──────────────────────────────────────────────

    let date = match &cli.date {
        Some(d) => NaiveDate::parse_from_str(d, "%Y-%m-%d").unwrap_or_else(|e| {
            eprintln!("Error: Invalid date '{}': {}", d, e);
            std::process::exit(1);
        }),
        None => Utc::now().naive_utc().date(),
    };

    // ── Apply timezone override ─────────────────────────────────

    let final_resolved = match &cli.tz {
        Some(tz_str) => {
            // Validate the timezone
            let _: Tz = tz_str.parse().unwrap_or_else(|_| {
                eprintln!("Error: Unknown timezone '{}'. Use IANA format (e.g. Europe/Oslo).", tz_str);
                std::process::exit(1);
            });
            ResolvedLocation {
                tz: tz_str.clone(),
                ..resolved
            }
        }
        None => resolved,
    };

    // ── Print location banner ───────────────────────────────────

    eprintln!("  {} {}", "\u{1F4CD}", final_resolved.display_line());
    if final_resolved.disambiguated {
        if let Some(ref note) = final_resolved.disambiguation_note {
            eprintln!("  \u{26A0}\u{FE0F}  Disambiguated: {}", note);
        }
    }

    // ── Solve ───────────────────────────────────────────────────

    let solver = Solver::from_resolved(&final_resolved).with_strategy(cli.strategy);
    let output = solver.solve_with_info(date, cli.now, cli.debug_wave, Some(&final_resolved));

    // ASCII timeline to stderr
    eprint!("{}", render_ascii_timeline(&output.events, output.state, output.gap_strategy, cli.show_confidence));

    // JSON to stdout
    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}

fn resolve_location(cli: &Cli, resolver: &mut LocationResolver, opts: &ResolveOptions) -> ResolvedLocation {
    // Priority: --city > positional city > --auto > --lat/--lon > error

    // 1. --city flag
    if let Some(ref city) = cli.city {
        return resolver.resolve_city_with_opts(city, opts).unwrap_or_else(|e| {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        });
    }

    // 2. Positional city argument
    if let Some(ref city) = cli.city_positional {
        return resolver.resolve_city_with_opts(city, opts).unwrap_or_else(|e| {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        });
    }

    // 3. --auto
    if cli.auto {
        return resolver.resolve_auto().unwrap_or_else(|e| {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        });
    }

    // 4. Legacy --lat/--lon
    if let (Some(lat), Some(lon)) = (cli.lat, cli.lon) {
        if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lon) {
            eprintln!("Error: Invalid coordinates. Lat: -90..90, Lon: -180..180");
            std::process::exit(1);
        }
        return LocationResolver::from_manual(lat, lon, cli.tz.as_deref());
    }

    // 5. Nothing provided
    eprintln!("Error: No location specified.");
    eprintln!();
    eprintln!("Usage:");
    eprintln!("  polaris Stockholm");
    eprintln!("  polaris --city \"New York\"");
    eprintln!("  polaris --city \"Medina, Saudi Arabia\"");
    eprintln!("  polaris --city Medina --country SA");
    eprintln!("  polaris --auto");
    eprintln!("  polaris --lat 21.4225 --lon 39.8262 --tz Asia/Riyadh");
    std::process::exit(1);
}
