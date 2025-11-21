//! HTTP callback execution module
//! Handles HTTP POST requests to external webhook URLs

use crate::models::{HTTPCallback, Timer};
use reqwest::Client;
use std::time::Duration;
use tracing::{info, warn};

/// Execute HTTP callback for a timer
///
/// Builds and sends an HTTP POST request with custom headers and JSON payload.
/// Returns Ok(()) on 2xx response, Err with error message otherwise.
pub async fn execute_http_callback(
    timer: &Timer,
    http_config: &HTTPCallback,
) -> Result<(), String> {
    // Build HTTP client with 30s timeout
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    // Build request
    let mut request = client
        .post(&http_config.url)
        .header("Content-Type", "application/json")
        .header("User-Agent", "timer-platform/0.1.0");

    // Add custom headers if present
    if let Some(headers) = &http_config.headers {
        if let Some(obj) = headers.as_object() {
            for (key, value) in obj {
                if let Some(val_str) = value.as_str() {
                    request = request.header(key, val_str);
                }
            }
        }
    }

    // Add payload if present
    if let Some(payload) = &http_config.payload {
        request = request.json(payload);
    }

    // Execute request
    match request.send().await {
        Ok(response) => {
            if response.status().is_success() {
                info!(
                    "HTTP callback succeeded for timer {}: {}",
                    timer.id,
                    response.status()
                );
                Ok(())
            } else {
                let error = format!("HTTP {} from {}", response.status(), http_config.url);
                warn!("HTTP callback failed for timer {}: {}", timer.id, error);
                Err(error)
            }
        }
        Err(e) => {
            let error = format!("HTTP request failed: {}", e);
            warn!("HTTP callback failed for timer {}: {}", timer.id, error);
            Err(error)
        }
    }
}
