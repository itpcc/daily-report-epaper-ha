use axum::{extract::State, Json};
use serde_json::{json, Value};

use crate::{api_error::ApiError, AppState};

pub async fn health_check() -> Result<Json<Value>, ApiError> {
    Ok(Json(json!({ "status": "ok" })))
}

pub async fn test(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    let weather = state.weather.read().await;

    Ok(Json(json!(*weather)))
}
