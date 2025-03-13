use std::sync::Arc;

use axum::Router;

pub mod api_error;
pub mod cfg;
pub mod cron;
pub mod db;
pub mod middleware;
pub mod model;
pub mod routes;
pub mod telemetry;

pub use cfg::*;
pub use db::*;
use model::{CalendarMap, WeatherInfoArc};
use time::PrimitiveDateTime;
use time_tz::{Tz, timezones};
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct AppState {
    pub db: Db,
    pub cfg: Config,
    pub tz: &'static Tz,
    pub calendar: Arc<RwLock<CalendarMap>>,
    pub weather: WeatherInfoArc,
    pub last_update: Arc<RwLock<PrimitiveDateTime>>,
}

pub fn router(
    cfg: Config,
    db: Db,
    calendar: Arc<RwLock<CalendarMap>>,
    weather: WeatherInfoArc,
    last_update: Arc<RwLock<PrimitiveDateTime>>,
) -> Router {
    let tz = timezones::get_by_name(&cfg.tz).unwrap_or(timezones::db::UTC);
    let app_state = AppState {
        db,
        cfg,
        tz,
        calendar,
        weather,
        last_update,
    };

    // Middleware that adds high level tracing to a Service.
    // Trace comes with good defaults but also supports customizing many aspects of the output:
    // https://docs.rs/tower-http/latest/tower_http/trace/index.html
    let trace_layer = telemetry::trace_layer();

    // Sets 'x-request-id' header with randomly generated uuid v7.
    let request_id_layer = middleware::request_id_layer();

    // Propagates 'x-request-id' header from the request to the response.
    let propagate_request_id_layer = middleware::propagate_request_id_layer();

    // Layer that applies the Cors middleware which adds headers for CORS.
    let cors_layer = middleware::cors_layer();

    // Layer that applies the Timeout middleware, which sets a timeout for requests.
    // The default value is 15 seconds.
    let timeout_layer = middleware::timeout_layer();

    // Any trailing slashes from request paths will be removed. For example, a request with `/foo/`
    // will be changed to `/foo` before reaching the internal service.
    let normalize_path_layer = middleware::normalize_path_layer();

    // Create the router with the routes.
    let router = routes::router();

    // Combine all the routes and apply the middleware layers.
    // The order of the layers is important. The first layer is the outermost layer.
    Router::new()
        .merge(router)
        .layer(normalize_path_layer)
        .layer(cors_layer)
        .layer(timeout_layer)
        .layer(propagate_request_id_layer)
        .layer(trace_layer)
        .layer(request_id_layer)
        .layer(axum::middleware::from_fn_with_state(
            app_state.clone(),
            middleware::auth_check_layer,
        ))
        .with_state(app_state)
}
