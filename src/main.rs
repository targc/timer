mod api_cancel_timer;
mod api_create_timer;
mod api_get_timer;
mod api_health;
mod api_list_timers;
mod api_update_timer;
mod auth;
mod callback;
mod callback_http;
mod callback_nats;
mod config;
mod db;
mod models;
mod scheduler;

use axum::{
    middleware,
    routing::{delete, get, post, put},
    Router,
};
use sqlx::postgres::PgPoolOptions;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::models::AppState;

#[tokio::main]
async fn main() {
    // Step 1: Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "timer=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting Timer Platform...");

    // Step 2: Load configuration
    let config = config::Config::from_env().expect("Failed to load configuration");

    tracing::info!("Configuration loaded successfully");
    tracing::info!("Database URL: {}", mask_password(&config.database_url));
    tracing::info!("Server port: {}", config.port);
    tracing::info!("Log level: {}", config.rust_log);

    // Step 3: Connect to database
    tracing::info!("Connecting to database...");
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&config.database_url)
        .await
        .expect("Failed to connect to database");

    tracing::info!("Database connection established");

    // Step 4: Run migrations
    tracing::info!("Running database migrations...");
    sqlx::migrate!()
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    tracing::info!("Database migrations completed");

    // Step 5: Initialize in-memory cache
    let timer_cache = Arc::new(RwLock::new(HashMap::new()));
    tracing::info!("In-memory cache initialized");

    // Step 6: Initialize NATS client (optional)
    let nats_client = if let Some(nats_config) = &config.nats_config {
        let nats_url = format!("nats://{}:{}", nats_config.host, nats_config.port);
        tracing::info!("Connecting to NATS at {}", nats_url);

        // Build connection options
        let mut options = async_nats::ConnectOptions::new();

        // Add authentication if credentials provided
        if let (Some(user), Some(password)) = (&nats_config.user, &nats_config.password) {
            tracing::info!("Using NATS authentication for user: {}", user);
            options = options.user_and_password(user.clone(), password.clone());
        }

        match options.connect(&nats_url).await {
            Ok(client) => {
                tracing::info!("NATS connection established");
                Some(client)
            }
            Err(e) => {
                tracing::error!("Failed to connect to NATS: {}", e);
                panic!("NATS connection failed: {}", e);
            }
        }
    } else {
        tracing::info!("NATS not configured, NATS callbacks disabled");
        None
    };

    // Step 7: Start scheduler
    scheduler::start_scheduler(pool.clone(), timer_cache.clone(), nats_client.clone());

    // Step 8: Create shared AppState
    let state = Arc::new(AppState {
        pool,
        config: config.clone(),
        timer_cache,
        nats_client,
    });

    // Step 9: Build router with protected and public routes
    let protected_routes = Router::new()
        .route("/timers", post(api_create_timer::create_timer))
        .route("/timers", get(api_list_timers::list_timers))
        .route("/timers/:id", get(api_get_timer::get_timer))
        .route("/timers/:id", put(api_update_timer::update_timer))
        .route("/timers/:id", delete(api_cancel_timer::cancel_timer))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::auth_middleware,
        ));

    let app = Router::new()
        .merge(protected_routes)
        .route("/healthz", get(api_health::health_check))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // Step 10: Start HTTP server
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::info!("Server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind server");

    axum::serve(listener, app)
        .await
        .expect("Failed to start server");
}

/// Mask password in database URL for logging
fn mask_password(url: &str) -> String {
    if let Some(at_pos) = url.rfind('@') {
        if let Some(colon_pos) = url[..at_pos].rfind(':') {
            let mut masked = url.to_string();
            masked.replace_range(colon_pos + 1..at_pos, "****");
            return masked;
        }
    }
    url.to_string()
}
