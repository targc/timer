use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::{Duration, Utc};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    db,
    models::{ApiResponse, AppState, CallbackConfig, CallbackType, TimerResponse, TimerStatus},
};

#[derive(Debug, Deserialize)]
pub struct UpdateTimerRequest {
    pub execute_at: Option<chrono::DateTime<Utc>>,
    pub callback: Option<CallbackConfig>,
    pub metadata: Option<serde_json::Value>,
}

pub async fn update_timer(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateTimerRequest>,
) -> Result<
    (StatusCode, Json<ApiResponse<TimerResponse>>),
    (StatusCode, Json<ApiResponse<()>>),
> {
    // Fetch existing timer to check status
    let existing_timer = match db::db_get_timer(&state.pool, id).await {
        Ok(Some(timer)) => timer,
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ApiResponse::<()>::error(3, "timer not found")),
            ));
        }
        Err(err) => {
            tracing::error!("Failed to get timer {}: {}", id, err);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error(1, format!("Database error: {}", err))),
            ));
        }
    };

    // Reject if status is completed, failed, or canceled
    if matches!(
        existing_timer.status,
        TimerStatus::Completed | TimerStatus::Failed | TimerStatus::Canceled
    ) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<()>::error(
                2,
                format!("cannot update timer with status '{}'", existing_timer.status),
            )),
        ));
    }

    // Validate new execute_at is in future if provided
    if let Some(execute_at) = req.execute_at {
        let now = Utc::now();
        let min_execute_time = now + Duration::seconds(5);

        if execute_at <= min_execute_time {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<()>::error(
                    2,
                    "execute_at must be at least 5 seconds in the future",
                )),
            ));
        }
    }

    // Validate callback configuration if provided
    let callback_type = if let Some(ref callback) = req.callback {
        match callback {
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
                Some(CallbackType::Http)
            }
            CallbackConfig::Nats(nats) => {
                // Validate NATS is available
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
                Some(CallbackType::Nats)
            }
        }
    } else {
        None
    };

    // Update timer
    match db::db_update_timer(
        &state.pool,
        id,
        req.execute_at,
        callback_type,
        req.callback,
        req.metadata,
    )
    .await
    {
        Ok(timer) => {
            let response = timer.to_response();
            Ok((StatusCode::OK, Json(ApiResponse::success(response))))
        }
        Err(err) => {
            tracing::error!("Failed to update timer {}: {}", id, err);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error(1, format!("Database error: {}", err))),
            ))
        }
    }
}
