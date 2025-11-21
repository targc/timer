use async_nats::Client as NatsClient;
use chrono::Utc;
use sqlx::PgPool;
use tokio::time::{interval, Duration};

use crate::callback::execute_callback;
use crate::db::{db_load_near_term_timers, db_mark_executing};
use crate::models::TimerCache;

/// Start the scheduler with two background tasks:
/// - Memory Loader (runs every 30s)
/// - Execution Task (runs every 1s)
pub fn start_scheduler(pool: PgPool, cache: TimerCache, nats_client: Option<NatsClient>) {
    // Clone for memory loader task
    let pool_loader = pool.clone();
    let cache_loader = cache.clone();

    // Clone for execution task
    let pool_executor = pool.clone();
    let cache_executor = cache.clone();
    let nats_executor = nats_client.clone();

    // Spawn Memory Loader Task (30s interval)
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(30));

        loop {
            interval.tick().await;

            match db_load_near_term_timers(&pool_loader).await {
                Ok(timers) => {
                    let count = timers.len();

                    // Acquire write lock and replace entire cache
                    let mut cache_guard = cache_loader.write().await;
                    cache_guard.clear();

                    for timer in timers {
                        cache_guard.insert(timer.id, timer);
                    }

                    // Lock released automatically when guard drops
                    tracing::info!("Loaded {} timers into cache", count);
                }
                Err(err) => {
                    tracing::warn!("Failed to load near-term timers: {}", err);
                }
            }
        }
    });

    // Spawn Execution Task (1s interval)
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(1));

        loop {
            interval.tick().await;

            let now = Utc::now();

            // Acquire read lock to find due timers
            let due_timers = {
                let cache_guard = cache_executor.read().await;
                cache_guard
                    .values()
                    .filter(|t| t.execute_at <= now)
                    .cloned()
                    .collect::<Vec<_>>()
            };
            // Read lock released here

            let count = due_timers.len();
            if count > 0 {
                tracing::info!("Executing {} due timers", count);
            }

            for timer in due_timers {
                let timer_id = timer.id;
                let pool_clone = pool_executor.clone();
                let nats_clone = nats_executor.clone();

                // Mark as executing in database
                match db_mark_executing(&pool_executor, timer_id).await {
                    Ok(_) => {
                        // Remove from cache
                        cache_executor.write().await.remove(&timer_id);

                        // Spawn async task to execute callback
                        tokio::spawn(async move {
                            tracing::info!("Spawned callback for timer {}", timer_id);

                            execute_callback(&pool_clone, timer, nats_clone.as_ref()).await;
                        });
                    }
                    Err(err) => {
                        tracing::warn!(
                            "Failed to mark timer {} as executing: {}",
                            timer_id,
                            err
                        );
                    }
                }
            }
        }
    });

    tracing::info!("Scheduler started with Memory Loader (30s) and Execution Task (1s)");
}
