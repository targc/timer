//! NATS callback execution module
//! Handles fire-and-forget message publishing to NATS topics

use crate::models::{NATSCallback, Timer};
use async_nats::Client as NatsClient;
use tracing::{info, warn};

/// Execute NATS callback for a timer
///
/// Publishes a message to the specified NATS topic with optional headers.
/// Returns Ok(()) on successful publish, Err with error message otherwise.
pub async fn execute_nats_callback(
    timer: &Timer,
    nats_config: &NATSCallback,
    nats_client: &NatsClient,
) -> Result<(), String> {
    // Build message payload
    let payload = if let Some(payload_value) = &nats_config.payload {
        serde_json::to_vec(payload_value)
            .map_err(|e| format!("Failed to serialize payload: {}", e))?
    } else {
        // Empty payload if none provided
        Vec::new()
    };

    // Build NATS subject (topic + optional key)
    let subject = if let Some(key) = &nats_config.key {
        format!("{}.{}", nats_config.topic, key)
    } else {
        nats_config.topic.clone()
    };

    // Create headers if present
    let headers = if let Some(headers_value) = &nats_config.headers {
        if let Some(headers_obj) = headers_value.as_object() {
            let mut nats_headers = async_nats::HeaderMap::new();
            for (key, value) in headers_obj {
                if let Some(val_str) = value.as_str() {
                    nats_headers.insert(key.as_str(), val_str);
                }
            }
            Some(nats_headers)
        } else {
            None
        }
    } else {
        None
    };

    // Publish message (fire-and-forget)
    let result = if let Some(hdrs) = headers {
        nats_client
            .publish_with_headers(subject.clone(), hdrs, payload.into())
            .await
    } else {
        nats_client.publish(subject.clone(), payload.into()).await
    };

    match result {
        Ok(_) => {
            info!(
                "NATS callback succeeded for timer {}: published to {}",
                timer.id, subject
            );
            Ok(())
        }
        Err(e) => {
            let error = format!("NATS publish failed: {}", e);
            warn!("NATS callback failed for timer {}: {}", timer.id, error);
            Err(error)
        }
    }
}
