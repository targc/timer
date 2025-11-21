use axum::{extract::State, http::StatusCode, Json};
use chrono::Utc;
use serde::Serialize;
use std::sync::Arc;

use crate::models::{ApiResponse, AppState};

#[derive(Debug, Serialize)]
pub struct HealthData {
    pub status: String,
    pub database: String,
    pub timestamp: chrono::DateTime<Utc>,
}

pub async fn health_check(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<ApiResponse<HealthData>>) {
    let timestamp = Utc::now();

    // Test database connection
    match sqlx::query("SELECT 1").fetch_one(&state.pool).await {
        Ok(_) => {
            let data = HealthData {
                status: "up".to_string(),
                database: "connected".to_string(),
                timestamp,
            };
            (StatusCode::OK, Json(ApiResponse::success(data)))
        }
        Err(err) => {
            tracing::error!("Health check failed: {}", err);
            let data = HealthData {
                status: "degraded".to_string(),
                database: "disconnected".to_string(),
                timestamp,
            };
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse {
                    code: 1,
                    message: "database connection failed".to_string(),
                    data: Some(data),
                }),
            )
        }
    }
}
