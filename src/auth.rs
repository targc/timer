use axum::{
    extract::State,
    http::{HeaderMap, Request, StatusCode},
    middleware::Next,
    response::Response,
    Json,
};
use std::sync::Arc;

use crate::models::{ApiResponse, AppState};

pub async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    req: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, (StatusCode, Json<ApiResponse<()>>)> {
    // Extract X-API-Key header
    let api_key = headers.get("X-API-Key").and_then(|v| v.to_str().ok());

    // Validate against configured API key
    if api_key != Some(&state.config.api_key) {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse {
                code: 4,
                message: "unauthorized".to_string(),
                data: None,
            }),
        ));
    }

    // Key is valid, proceed to handler
    Ok(next.run(req).await)
}
