//! Callback execution dispatcher module
//! Routes callback execution to either HTTP or NATS based on callback_type

use crate::callback_http::execute_http_callback;
use crate::callback_nats::execute_nats_callback;
use crate::db::{db_mark_completed, db_mark_failed};
use crate::models::{CallbackConfig, Timer};
use async_nats::Client as NatsClient;
use sqlx::PgPool;
use tracing::{info, warn};

/// Execute callback for a timer (dispatcher)
///
/// Routes to the appropriate callback handler based on callback_config.
/// Updates timer status in database based on execution result.
pub async fn execute_callback(pool: &PgPool, timer: Timer, nats_client: Option<&NatsClient>) {
    info!("Executing callback for timer {}", timer.id);

    // Dispatch to appropriate callback handler
    let result = match &timer.callback_config {
        CallbackConfig::Http(http_config) => execute_http_callback(&timer, http_config).await,
        CallbackConfig::Nats(nats_config) => {
            if let Some(client) = nats_client {
                execute_nats_callback(&timer, nats_config, client).await
            } else {
                Err("NATS client not available (NATS_URL not configured)".to_string())
            }
        }
    };

    // Update timer status based on result
    match result {
        Ok(_) => {
            info!("Callback completed successfully for timer {}", timer.id);
            if let Err(e) = db_mark_completed(pool, timer.id).await {
                warn!("Failed to mark timer as completed: {}", e);
            }
        }
        Err(error_msg) => {
            warn!("Callback failed for timer {}: {}", timer.id, error_msg);
            if let Err(e) = db_mark_failed(pool, timer.id, error_msg).await {
                warn!("Failed to mark timer as failed: {}", e);
            }
        }
    }
}
