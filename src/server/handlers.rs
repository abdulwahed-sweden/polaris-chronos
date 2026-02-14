use axum::extract::{Query, State};
use axum::http::{header, StatusCode};
use axum::response::{Html, IntoResponse, Json, Response};
use chrono::{NaiveDate, Utc};
use serde::Deserialize;
use std::sync::Arc;

use crate::location::{builtin_city_list, ResolveOptions};
use crate::schedule::GapStrategy;
use crate::solver::Solver;

use super::state::AppState;
use super::static_files;

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

// ─── API handlers ────────────────────────────────────────────────

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
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    // Resolve location
    let resolved = if let Some(ref city) = params.city {
        let opts = ResolveOptions {
            country: params.country.clone(),
            topk: None,
        };
        let mut resolver = state.resolver.lock().unwrap();
        resolver.resolve_city_with_opts(city, &opts).map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": format!("{}", e) })),
            )
        })?
    } else if let (Some(lat), Some(lon)) = (params.lat, params.lon) {
        if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lon) {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "Invalid coordinates. Lat: -90..90, Lon: -180..180" })),
            ));
        }
        crate::location::LocationResolver::from_manual(lat, lon, params.tz.as_deref())
    } else {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Provide 'city' or 'lat'+'lon' parameters" })),
        ));
    };

    // Apply timezone override
    let final_resolved = if let Some(ref tz_str) = params.tz {
        let _: chrono_tz::Tz = tz_str.parse().map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": format!("Unknown timezone '{}'", tz_str) })),
            )
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
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": format!("Invalid date '{}': {}", d, e) })),
            )
        })?,
        None => Utc::now().naive_utc().date(),
    };

    // Parse strategy
    let strategy = match params.strategy.as_deref() {
        Some("strict") => GapStrategy::Strict,
        Some("projected45") | Some("projected") | None => GapStrategy::Projected45,
        Some(s) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": format!("Unknown strategy '{}'. Use 'strict' or 'projected45'.", s) })),
            ));
        }
    };

    // Solve
    let solver = Solver::from_resolved(&final_resolved).with_strategy(strategy);
    let output = solver.solve_with_info(date, false, false, Some(&final_resolved));

    Ok(Json(output))
}

pub async fn city_list() -> Json<Vec<crate::location::CityInfo>> {
    Json(builtin_city_list())
}
