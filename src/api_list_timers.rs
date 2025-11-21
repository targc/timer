use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{
    db,
    models::{ApiResponse, AppState, TimerResponse},
};

#[derive(Debug, Deserialize)]
pub struct ListTimersQuery {
    pub status: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub sort: Option<String>,
    pub order: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ListTimersResponse {
    pub timers: Vec<TimerResponse>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

pub async fn list_timers(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ListTimersQuery>,
) -> Result<
    (StatusCode, Json<ApiResponse<ListTimersResponse>>),
    (StatusCode, Json<ApiResponse<()>>),
> {
    // Set defaults
    let limit = params.limit.unwrap_or(50).min(200).max(1);
    let offset = params.offset.unwrap_or(0).max(0);
    let sort_field = params.sort.as_deref().unwrap_or("created_at");
    let sort_order = params.order.as_deref().unwrap_or("desc");

    // Validate sort field
    if !matches!(sort_field, "created_at" | "execute_at") {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<()>::error(
                2,
                "sort field must be 'created_at' or 'execute_at'",
            )),
        ));
    }

    // Validate sort order
    if !matches!(sort_order, "asc" | "desc") {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<()>::error(2, "order must be 'asc' or 'desc'")),
        ));
    }

    // Validate status filter if provided
    if let Some(status) = &params.status {
        if !matches!(
            status.as_str(),
            "pending" | "executing" | "completed" | "failed" | "canceled"
        ) {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<()>::error(
                    2,
                    "status must be one of: pending, executing, completed, failed, canceled",
                )),
            ));
        }
    }

    match db::db_list_timers(
        &state.pool,
        params.status.clone(),
        limit,
        offset,
        sort_field,
        sort_order,
    )
    .await
    {
        Ok((timers, total)) => {
            let timer_responses: Vec<TimerResponse> =
                timers.iter().map(|t| t.to_response()).collect();

            let response = ListTimersResponse {
                timers: timer_responses,
                total,
                limit,
                offset,
            };

            Ok((StatusCode::OK, Json(ApiResponse::success(response))))
        }
        Err(err) => {
            tracing::error!("Failed to list timers: {}", err);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error(1, format!("Database error: {}", err))),
            ))
        }
    }
}
