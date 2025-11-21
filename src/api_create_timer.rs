use axum::{extract::State, http::StatusCode, Json};
use chrono::{Duration, Utc};
use serde::Deserialize;
use std::sync::Arc;

use crate::{
    db,
    models::{ApiResponse, AppState, CallbackConfig, CallbackType, TimerResponse},
};

#[derive(Debug, Deserialize)]
pub struct CreateTimerRequest {
    pub execute_at: chrono::DateTime<Utc>,
    pub callback: CallbackConfig,
    pub metadata: Option<serde_json::Value>,
}

pub async fn create_timer(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateTimerRequest>,
) -> Result<
    (StatusCode, Json<ApiResponse<TimerResponse>>),
    (StatusCode, Json<ApiResponse<()>>),
> {
    // Validate execute_at is in future (> NOW + 5 seconds)
    let now = Utc::now();
    let min_execute_time = now + Duration::seconds(5);

    if req.execute_at <= min_execute_time {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<()>::error(
                2,
                "execute_at must be at least 5 seconds in the future",
            )),
        ));
    }

    // Validate callback configuration
    match &req.callback {
        CallbackConfig::Http(http) => {
            if !http.url.starts_with("http://") && !http.url.starts_with("https://") {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::<()>::error(
                        2,
                        "HTTP callback URL must start with http:// or https://",
                    )),
                ));
            }
        }
        CallbackConfig::Nats(nats) => {
            // Validate NATS is available if requested
            if state.nats_client.is_none() {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::<()>::error(
                        2,
                        "NATS callbacks not available (NATS_URL not configured)",
                    )),
                ));
            }
            // Validate topic is not empty
            if nats.topic.trim().is_empty() {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::<()>::error(
                        2,
                        "NATS topic cannot be empty",
                    )),
                ));
            }
        }
    }

    // Determine callback_type
    let callback_type = match &req.callback {
        CallbackConfig::Http(_) => CallbackType::Http,
        CallbackConfig::Nats(_) => CallbackType::Nats,
    };

    // Create timer in database
    match db::db_create_timer(
        &state.pool,
        req.execute_at,
        callback_type,
        req.callback,
        req.metadata,
    )
    .await
    {
        Ok(timer) => {
            let response = timer.to_response();
            Ok((
                StatusCode::CREATED,
                Json(ApiResponse::success(response)),
            ))
        }
        Err(err) => {
            tracing::error!("Failed to create timer: {}", err);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error(1, format!("Database error: {}", err))),
            ))
        }
    }
}
