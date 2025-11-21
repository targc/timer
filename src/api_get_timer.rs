use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    db,
    models::{ApiResponse, AppState, CallbackConfig},
};

#[derive(Debug, Serialize)]
pub struct TimerDetailResponse {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub execute_at: DateTime<Utc>,
    pub callback: CallbackConfig,
    pub status: String,
    pub last_error: Option<String>,
    pub executed_at: Option<DateTime<Utc>>,
    pub metadata: Option<serde_json::Value>,
}

pub async fn get_timer(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<
    (StatusCode, Json<ApiResponse<TimerDetailResponse>>),
    (StatusCode, Json<ApiResponse<()>>),
> {
    match db::db_get_timer(&state.pool, id).await {
        Ok(Some(timer)) => {
            let response = TimerDetailResponse {
                id: timer.id,
                created_at: timer.created_at,
                updated_at: timer.updated_at,
                execute_at: timer.execute_at,
                callback: timer.callback_config,
                status: timer.status.to_string(),
                last_error: timer.last_error,
                executed_at: timer.executed_at,
                metadata: timer.metadata,
            };
            Ok((StatusCode::OK, Json(ApiResponse::success(response))))
        }
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<()>::error(3, "timer not found")),
        )),
        Err(err) => {
            tracing::error!("Failed to get timer {}: {}", id, err);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error(1, format!("Database error: {}", err))),
            ))
        }
    }
}
