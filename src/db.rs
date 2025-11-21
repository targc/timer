use anyhow::Result;
use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::models::{CallbackConfig, CallbackType, Timer};

/// Create a new timer
pub async fn db_create_timer(
    pool: &PgPool,
    execute_at: DateTime<Utc>,
    callback_type: CallbackType,
    callback_config: CallbackConfig,
    metadata: Option<Value>,
) -> Result<Timer> {
    // Serialize callback_config to JSON
    let callback_config_json = serde_json::to_value(&callback_config)?;

    let timer = sqlx::query_as::<_, Timer>(
        r#"
        INSERT INTO timers (
            id, execute_at, callback_type, callback_config, metadata, status
        )
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING
            id, created_at, updated_at, execute_at, callback_type,
            callback_config, status, last_error, executed_at, metadata
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(execute_at)
    .bind(callback_type)
    .bind(callback_config_json)
    .bind(metadata)
    .bind("pending")
    .fetch_one(pool)
    .await?;

    Ok(timer)
}

/// Get timer by ID
pub async fn db_get_timer(pool: &PgPool, timer_id: Uuid) -> Result<Option<Timer>> {
    let timer = sqlx::query_as::<_, Timer>(
        r#"
        SELECT
            id, created_at, updated_at, execute_at, callback_type,
            callback_config, status, last_error, executed_at, metadata
        FROM timers
        WHERE id = $1
        "#,
    )
    .bind(timer_id)
    .fetch_optional(pool)
    .await?;

    Ok(timer)
}

/// List timers with filtering, sorting, and pagination
pub async fn db_list_timers(
    pool: &PgPool,
    status_filter: Option<String>,
    limit: i64,
    offset: i64,
    sort_field: &str,
    sort_order: &str,
) -> Result<(Vec<Timer>, i64)> {
    // Build the WHERE clause
    let where_clause = if let Some(status) = &status_filter {
        format!("WHERE status = '{}'", status)
    } else {
        String::new()
    };

    // Build ORDER BY clause
    let order_clause = format!("ORDER BY {} {}", sort_field, sort_order);

    // Build full query
    let query = format!(
        r#"
        SELECT
            id, created_at, updated_at, execute_at, callback_type,
            callback_config, status, last_error, executed_at, metadata
        FROM timers
        {}
        {}
        LIMIT $1 OFFSET $2
        "#,
        where_clause, order_clause
    );

    let timers = sqlx::query_as::<_, Timer>(&query)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;

    // Get total count
    let count_query = format!("SELECT COUNT(*) as count FROM timers {}", where_clause);

    let row = sqlx::query(&count_query).fetch_one(pool).await?;
    let total: i64 = row.try_get("count")?;

    Ok((timers, total))
}

/// Update timer fields
pub async fn db_update_timer(
    pool: &PgPool,
    timer_id: Uuid,
    execute_at: Option<DateTime<Utc>>,
    callback_type: Option<CallbackType>,
    callback_config: Option<CallbackConfig>,
    metadata: Option<Value>,
) -> Result<Timer> {
    // Build dynamic update query
    let mut updates: Vec<String> = vec!["updated_at = NOW()".to_string()];
    let mut param_index = 2; // $1 is timer_id

    if execute_at.is_some() {
        updates.push(format!("execute_at = ${}", param_index));
        param_index += 1;
    }
    if callback_type.is_some() {
        updates.push(format!("callback_type = ${}", param_index));
        param_index += 1;
    }
    if callback_config.is_some() {
        updates.push(format!("callback_config = ${}", param_index));
        param_index += 1;
    }
    if metadata.is_some() {
        updates.push(format!("metadata = ${}", param_index));
    }

    let query = format!(
        r#"UPDATE timers SET {} WHERE id = $1
        RETURNING
            id, created_at, updated_at, execute_at, callback_type,
            callback_config, status, last_error, executed_at, metadata
        "#,
        updates.join(", ")
    );

    // Build and execute query with bindings
    let mut q = sqlx::query_as::<_, Timer>(&query).bind(timer_id);

    if let Some(ea) = execute_at {
        q = q.bind(ea);
    }
    if let Some(ct) = callback_type {
        q = q.bind(ct);
    }
    if let Some(cc) = callback_config {
        let cc_json = serde_json::to_value(&cc)?;
        q = q.bind(cc_json);
    }
    if let Some(meta) = metadata {
        q = q.bind(meta);
    }

    let timer = q.fetch_one(pool).await?;
    Ok(timer)
}

/// Cancel a timer (soft delete)
pub async fn db_cancel_timer(pool: &PgPool, timer_id: Uuid) -> Result<Timer> {
    let timer = sqlx::query_as::<_, Timer>(
        r#"
        UPDATE timers
        SET status = $2, updated_at = NOW()
        WHERE id = $1
        RETURNING
            id, created_at, updated_at, execute_at, callback_type,
            callback_config, status, last_error, executed_at, metadata
        "#,
    )
    .bind(timer_id)
    .bind("canceled")
    .fetch_one(pool)
    .await?;

    Ok(timer)
}

/// Load near-term timers for scheduler
/// Loads timers from NOW() - 5 minutes to NOW() + 1 minute
pub async fn db_load_near_term_timers(pool: &PgPool) -> Result<Vec<Timer>> {
    let timers = sqlx::query_as::<_, Timer>(
        r#"
        SELECT
            id, created_at, updated_at, execute_at, callback_type,
            callback_config, status, last_error, executed_at, metadata
        FROM timers
        WHERE status = $1
        AND execute_at > NOW() - INTERVAL '5 minutes'
        AND execute_at <= NOW() + INTERVAL '1 minute'
        ORDER BY execute_at ASC
        "#,
    )
    .bind("pending")
    .fetch_all(pool)
    .await?;

    Ok(timers)
}

/// Mark timer as executing
pub async fn db_mark_executing(pool: &PgPool, timer_id: Uuid) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE timers
        SET status = $2, updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(timer_id)
    .bind("executing")
    .execute(pool)
    .await?;

    Ok(())
}

/// Mark timer as completed
pub async fn db_mark_completed(pool: &PgPool, timer_id: Uuid) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE timers
        SET status = $2, executed_at = NOW(), updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(timer_id)
    .bind("completed")
    .execute(pool)
    .await?;

    Ok(())
}

/// Mark timer as failed with error message
pub async fn db_mark_failed(pool: &PgPool, timer_id: Uuid, error_message: String) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE timers
        SET status = $2, last_error = $3, executed_at = NOW(), updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(timer_id)
    .bind("failed")
    .bind(error_message)
    .execute(pool)
    .await?;

    Ok(())
}
