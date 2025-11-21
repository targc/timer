use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Serialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    db,
    models::{ApiResponse, AppState, TimerStatus},
};

#[derive(Debug, Serialize)]
pub struct CancelTimerResponse {
    pub id: Uuid,
    pub status: String,
}

pub async fn cancel_timer(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<
    (StatusCode, Json<ApiResponse<CancelTimerResponse>>),
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

    // Reject if status is completed or failed
    if matches!(
        existing_timer.status,
        TimerStatus::Completed | TimerStatus::Failed
    ) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<()>::error(
                2,
                format!(
                    "cannot cancel timer with status '{}'",
                    existing_timer.status
                ),
            )),
        ));
    }

    // Cancel timer
    match db::db_cancel_timer(&state.pool, id).await {
        Ok(timer) => {
            let response = CancelTimerResponse {
                id: timer.id,
                status: timer.status.to_string(),
            };
            Ok((StatusCode::OK, Json(ApiResponse::success(response))))
        }
        Err(err) => {
            tracing::error!("Failed to cancel timer {}: {}", id, err);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error(1, format!("Database error: {}", err))),
            ))
        }
    }
}
