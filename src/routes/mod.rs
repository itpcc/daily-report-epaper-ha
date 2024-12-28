use axum::{routing::get, Router};

pub mod epaper_page;
pub mod health_check;

use crate::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/health_check", get(health_check::health_check))
        .route("/last_update", get(health_check::last_update))
        .route("/epaper_page", get(epaper_page::epaper_page))
        .route("/test", get(health_check::test))
}
