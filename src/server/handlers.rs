use axum::extract::{Query, State};
use axum::http::{header, StatusCode};
use axum::response::{Html, IntoResponse, Json, Response};
use chrono::{Datelike, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

use crate::location::{builtin_city_list, ResolveOptions, country_display_name, format_coords};
use crate::location::types::LocationError;
use crate::schedule::GapStrategy;
use crate::solver::Solver;

use super::state::{AppState, ComputeCache};
use super::static_files;

// ─── Error response ──────────────────────────────────────────────

#[derive(Serialize)]
struct ApiErrorBody {
    error: String,
    code: u16,
}

pub(super) struct ApiError(StatusCode, String);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = ApiErrorBody {
            error: self.1,
            code: self.0.as_u16(),
        };
        (self.0, Json(body)).into_response()
    }
}

fn api_error(status: StatusCode, msg: impl Into<String>) -> ApiError {
    ApiError(status, msg.into())
}

// ─── Static file handlers ────────────────────────────────────────

pub async fn index() -> Html<&'static str> {
    Html(static_files::INDEX_HTML)
}

pub async fn style() -> Response {
    (
        [(header::CONTENT_TYPE, "text/css")],
        static_files::STYLE_CSS,
    )
        .into_response()
}

pub async fn script() -> Response {
    (
        [(header::CONTENT_TYPE, "application/javascript")],
        static_files::APP_JS,
    )
        .into_response()
}

// ─── GET /api/resolve ────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ResolveQuery {
    pub query: Option<String>,
    pub country: Option<String>,
}

#[derive(Serialize)]
pub struct ResolveResponse {
    pub name: String,
    pub lat: f64,
    pub lon: f64,
    pub tz: String,
    pub tz_label: String,
    pub country_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    pub formatted_coords: String,
    pub source: String,
    pub confidence: f64,
}

#[derive(Serialize)]
struct AmbiguousOption {
    name: String,
    country: String,
    country_code: String,
    tz: String,
    lat: f64,
    lon: f64,
}

#[derive(Serialize)]
struct AmbiguousResponse {
    multiple: bool,
    query: String,
    options: Vec<AmbiguousOption>,
}

pub async fn resolve(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ResolveQuery>,
) -> Result<Json<ResolveResponse>, Response> {
    let start = Instant::now();

    let query = params.query.as_deref().unwrap_or("").trim();
    if query.is_empty() {
        return Err(api_error(StatusCode::BAD_REQUEST, "Missing 'query' parameter").into_response());
    }

    let opts = ResolveOptions {
        country: params.country.clone(),
        topk: None,
    };

    let resolved = {
        let mut resolver = state.resolver.lock().unwrap();
        resolver.resolve_city_with_opts(query, &opts)
    };

    let resolved = match resolved {
        Ok(r) => r,
        Err(LocationError::Ambiguous { query: q, candidates }) => {
            let resp = AmbiguousResponse {
                multiple: true,
                query: q,
                options: candidates.iter().map(|c| AmbiguousOption {
                    name: c.name.clone(),
                    country: c.country_name.clone(),
                    country_code: c.country.clone(),
                    tz: c.tz.clone(),
                    lat: c.lat,
                    lon: c.lon,
                }).collect(),
            };
            return Err((StatusCode::MULTIPLE_CHOICES, Json(resp)).into_response());
        }
        Err(e) => {
            return Err(api_error(StatusCode::NOT_FOUND, format!("{}", e)).into_response());
        }
    };

    let elapsed = start.elapsed();
    eprintln!("[{}] GET /api/resolve?query={} -> {} ({:.1}ms)",
        Utc::now().format("%H:%M:%S"),
        query,
        resolved.name,
        elapsed.as_secs_f64() * 1000.0,
    );

    let country = resolved.country_code.as_deref().and_then(|cc| {
        let name = country_display_name(cc);
        if name == cc { None } else { Some(name.to_string()) }
    });

    Ok(Json(ResolveResponse {
        name: resolved.name.clone(),
        lat: resolved.lat,
        lon: resolved.lon,
        tz: resolved.tz.clone(),
        tz_label: format!("{} (Local Time)", resolved.tz),
        country_code: resolved.country_code.clone(),
        country,
        formatted_coords: format_coords(resolved.lat, resolved.lon),
        source: format!("{}", resolved.source),
        confidence: resolved.resolver_confidence,
    }))
}

// ─── GET /api/times ──────────────────────────────────────────────

#[derive(Deserialize)]
pub struct TimesQuery {
    pub city: Option<String>,
    pub country: Option<String>,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    pub tz: Option<String>,
    pub date: Option<String>,
    pub strategy: Option<String>,
}

pub async fn prayer_times(
    State(state): State<Arc<AppState>>,
    Query(params): Query<TimesQuery>,
) -> Result<impl IntoResponse, Response> {
    let start = Instant::now();

    // Resolve location
    let resolved = if let Some(ref city) = params.city {
        let opts = ResolveOptions {
            country: params.country.clone(),
            topk: None,
        };
        let mut resolver = state.resolver.lock().unwrap();
        match resolver.resolve_city_with_opts(city, &opts) {
            Ok(r) => r,
            Err(LocationError::Ambiguous { query, candidates }) => {
                let resp = AmbiguousResponse {
                    multiple: true,
                    query,
                    options: candidates.iter().map(|c| AmbiguousOption {
                        name: c.name.clone(),
                        country: c.country_name.clone(),
                        country_code: c.country.clone(),
                        tz: c.tz.clone(),
                        lat: c.lat,
                        lon: c.lon,
                    }).collect(),
                };
                return Err((StatusCode::MULTIPLE_CHOICES, Json(resp)).into_response());
            }
            Err(e) => return Err(api_error(StatusCode::NOT_FOUND, format!("{}", e)).into_response()),
        }
    } else if let (Some(lat), Some(lon)) = (params.lat, params.lon) {
        if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lon) {
            return Err(api_error(StatusCode::BAD_REQUEST,
                "Invalid coordinates. Lat: -90..90, Lon: -180..180").into_response());
        }
        crate::location::LocationResolver::from_manual(lat, lon, params.tz.as_deref())
    } else {
        return Err(api_error(StatusCode::BAD_REQUEST,
            "Provide 'city' or 'lat'+'lon' parameters").into_response());
    };

    // Apply timezone override
    let final_resolved = if let Some(ref tz_str) = params.tz {
        let _: chrono_tz::Tz = tz_str.parse().map_err(|_| {
            api_error(StatusCode::BAD_REQUEST, format!("Unknown timezone '{}'", tz_str)).into_response()
        })?;
        crate::location::ResolvedLocation {
            tz: tz_str.clone(),
            ..resolved
        }
    } else {
        resolved
    };

    // Parse date
    let date = match &params.date {
        Some(d) => NaiveDate::parse_from_str(d, "%Y-%m-%d").map_err(|e| {
            api_error(StatusCode::BAD_REQUEST, format!("Invalid date '{}': {}", d, e)).into_response()
        })?,
        None => Utc::now().naive_utc().date(),
    };

    // Parse strategy
    let strategy = parse_strategy(params.strategy.as_deref()).map_err(|e| e.into_response())?;
    let strategy_str = format!("{}", strategy);

    // Check cache
    let cache_key = ComputeCache::key(
        final_resolved.lat, final_resolved.lon,
        &date.to_string(), &strategy_str,
    );

    {
        let mut cache = state.cache.lock().unwrap();
        if let Some(cached) = cache.get(&cache_key) {
            let elapsed = start.elapsed();
            eprintln!("[{}] GET /api/times city={} date={} -> CACHED ({:.1}ms)",
                Utc::now().format("%H:%M:%S"),
                final_resolved.name, date,
                elapsed.as_secs_f64() * 1000.0,
            );
            return Ok(Json(cached));
        }
    }

    // Solve
    let solver = Solver::from_resolved(&final_resolved).with_strategy(strategy);
    let output = solver.solve_with_info(date, false, false, Some(&final_resolved));

    // Store in cache
    {
        let mut cache = state.cache.lock().unwrap();
        cache.put(cache_key, output.clone());
    }

    let elapsed = start.elapsed();
    eprintln!("[{}] GET /api/times city={} date={} -> {} ({:.1}ms)",
        Utc::now().format("%H:%M:%S"),
        final_resolved.name, date, output.state,
        elapsed.as_secs_f64() * 1000.0,
    );

    Ok(Json(output))
}

// ─── GET /api/month ──────────────────────────────────────────────

#[derive(Deserialize)]
pub struct MonthQuery {
    pub city: Option<String>,
    pub country: Option<String>,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    pub tz: Option<String>,
    pub year: Option<i32>,
    pub month: Option<u32>,
    pub strategy: Option<String>,
}

pub async fn month_times(
    State(state): State<Arc<AppState>>,
    Query(params): Query<MonthQuery>,
) -> Result<impl IntoResponse, Response> {
    let start = Instant::now();

    // Resolve location
    let resolved = if let Some(ref city) = params.city {
        let opts = ResolveOptions {
            country: params.country.clone(),
            topk: None,
        };
        let mut resolver = state.resolver.lock().unwrap();
        match resolver.resolve_city_with_opts(city, &opts) {
            Ok(r) => r,
            Err(LocationError::Ambiguous { query, candidates }) => {
                let resp = AmbiguousResponse {
                    multiple: true,
                    query,
                    options: candidates.iter().map(|c| AmbiguousOption {
                        name: c.name.clone(),
                        country: c.country_name.clone(),
                        country_code: c.country.clone(),
                        tz: c.tz.clone(),
                        lat: c.lat,
                        lon: c.lon,
                    }).collect(),
                };
                return Err((StatusCode::MULTIPLE_CHOICES, Json(resp)).into_response());
            }
            Err(e) => return Err(api_error(StatusCode::NOT_FOUND, format!("{}", e)).into_response()),
        }
    } else if let (Some(lat), Some(lon)) = (params.lat, params.lon) {
        if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lon) {
            return Err(api_error(StatusCode::BAD_REQUEST,
                "Invalid coordinates. Lat: -90..90, Lon: -180..180").into_response());
        }
        crate::location::LocationResolver::from_manual(lat, lon, params.tz.as_deref())
    } else {
        return Err(api_error(StatusCode::BAD_REQUEST,
            "Provide 'city' or 'lat'+'lon' parameters").into_response());
    };

    // Apply timezone override
    let final_resolved = if let Some(ref tz_str) = params.tz {
        let _: chrono_tz::Tz = tz_str.parse().map_err(|_| {
            api_error(StatusCode::BAD_REQUEST, format!("Unknown timezone '{}'", tz_str)).into_response()
        })?;
        crate::location::ResolvedLocation {
            tz: tz_str.clone(),
            ..resolved
        }
    } else {
        resolved
    };

    let today = Utc::now().naive_utc().date();
    let year = params.year.unwrap_or(today.year());
    let month = params.month.unwrap_or(today.month());

    if !(1..=12).contains(&month) {
        return Err(api_error(StatusCode::BAD_REQUEST, "Month must be 1-12").into_response());
    }

    let strategy = parse_strategy(params.strategy.as_deref()).map_err(|e| e.into_response())?;
    let strategy_str = format!("{}", strategy);

    // Compute all days in the month
    let first = NaiveDate::from_ymd_opt(year, month, 1)
        .ok_or_else(|| api_error(StatusCode::BAD_REQUEST, format!("Invalid year/month: {}/{}", year, month)).into_response())?;

    let days_in_month = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    }.unwrap().signed_duration_since(first).num_days() as u32;

    let solver = Solver::from_resolved(&final_resolved).with_strategy(strategy);
    let mut results = Vec::with_capacity(days_in_month as usize);
    let mut cache = state.cache.lock().unwrap();

    for day in 1..=days_in_month {
        let date = NaiveDate::from_ymd_opt(year, month, day).unwrap();
        let cache_key = ComputeCache::key(
            final_resolved.lat, final_resolved.lon,
            &date.to_string(), &strategy_str,
        );

        if let Some(cached) = cache.get(&cache_key) {
            results.push(cached);
        } else {
            let output = solver.solve_with_info(date, false, false, Some(&final_resolved));
            cache.put(cache_key, output.clone());
            results.push(output);
        }
    }

    let elapsed = start.elapsed();
    eprintln!("[{}] GET /api/month city={} {}/{} -> {} days ({:.1}ms)",
        Utc::now().format("%H:%M:%S"),
        final_resolved.name, year, month,
        days_in_month,
        elapsed.as_secs_f64() * 1000.0,
    );

    Ok(Json(results))
}

// ─── GET /api/cities ─────────────────────────────────────────────

pub async fn city_list() -> Json<Vec<crate::location::CityInfo>> {
    Json(builtin_city_list())
}

// ─── Helpers ─────────────────────────────────────────────────────

fn parse_strategy(s: Option<&str>) -> Result<GapStrategy, ApiError> {
    match s {
        Some("strict") => Ok(GapStrategy::Strict),
        Some("projected45") | Some("projected") | None => Ok(GapStrategy::Projected45),
        Some(other) => Err(api_error(
            StatusCode::BAD_REQUEST,
            format!("Unknown strategy '{}'. Use 'strict' or 'projected45'.", other),
        )),
    }
}
