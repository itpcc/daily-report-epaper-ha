use axum::{Json, extract::State};
use serde_json::{Value, json};

use crate::{AppState, api_error::ApiError};

pub async fn health_check() -> Result<Json<Value>, ApiError> {
    Ok(Json(json!({ "status": "ok" })))
}

pub async fn last_update(State(state): State<AppState>) -> Result<String, ApiError> {
    let last_update = state.last_update.read().await;

    Ok((*last_update).to_string())
}

pub async fn test(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    let weather = state.weather.read().await;

    Ok(Json(json!(*weather)))
}
