use std::time::Duration;

use axum::{
    extract::{Query, Request, State},
    http::{self, HeaderName, StatusCode},
    middleware::Next,
    response::Response,
};
use serde::Deserialize;
use tower_http::{
    cors::{AllowHeaders, Any, CorsLayer},
    normalize_path::NormalizePathLayer,
    request_id::{MakeRequestId, PropagateRequestIdLayer, RequestId, SetRequestIdLayer},
    timeout::TimeoutLayer,
};

use crate::AppState;

#[derive(Clone, Default)]
pub struct Id;

#[derive(Clone, Default, Deserialize)]
pub struct QueryAuthModel {
    pub token: String,
}

impl MakeRequestId for Id {
    fn make_request_id<B>(&mut self, _: &Request<B>) -> Option<RequestId> {
        let id = uuid::Uuid::now_v7().to_string().parse().unwrap();
        Some(RequestId::new(id))
    }
}

/// Sets the 'x-request-id' header with a randomly generated UUID v7.
///
/// SetRequestId will not override request IDs if they are already present
/// on requests or responses.
pub fn request_id_layer() -> SetRequestIdLayer<Id> {
    let x_request_id = HeaderName::from_static("x-request-id");
    SetRequestIdLayer::new(x_request_id.clone(), Id)
}

// Propagates 'x-request-id' header from the request to the response.
///
/// PropagateRequestId wont override request ids if its already
/// present on requests or responses.
pub fn propagate_request_id_layer() -> PropagateRequestIdLayer {
    let x_request_id = HeaderName::from_static("x-request-id");
    PropagateRequestIdLayer::new(x_request_id)
}

/// Layer that applies the Cors middleware which adds headers for CORS.
pub fn cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(AllowHeaders::mirror_request())
        .max_age(Duration::from_secs(600))
}

/// Layer that applies the Timeout middleware which apply a timeout to requests.
/// The default timeout value is set to 15 seconds.
pub fn timeout_layer() -> TimeoutLayer {
    TimeoutLayer::new(Duration::from_secs(15))
}

/// Middleware that normalizes paths.
///
/// Any trailing slashes from request paths will be removed. For example, a request with `/foo/`
/// will be changed to `/foo` before reaching the inner service.
pub fn normalize_path_layer() -> NormalizePathLayer {
    NormalizePathLayer::trim_trailing_slash()
}

/// Middleware for authorization check
pub async fn auth_check_layer(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header = req
        .headers()
        .get(http::header::AUTHORIZATION)
        .and_then(|header| header.to_str().ok());
    let auth_q = Query::<QueryAuthModel>::try_from_uri(req.uri());

    let auth_token = if let Some(auth_header) = auth_header {
        auth_header
    } else if let Ok(aq) = auth_q.as_ref() {
        aq.token.as_ref()
    } else {
        return Err(StatusCode::UNAUTHORIZED);
    };

    if auth_token != state.cfg.access_token {
        return Err(StatusCode::UNAUTHORIZED);
    }

    Ok(next.run(req).await)
}
