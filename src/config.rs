use anyhow::{anyhow, Context, Result};
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub api_key: String,
    pub port: u16,
    pub rust_log: String,
    /// Optional NATS configuration for pub/sub callbacks
    pub nats_config: Option<NatsConfig>,
}

#[derive(Debug, Clone)]
pub struct NatsConfig {
    pub host: String,
    pub port: u16,
    pub user: Option<String>,
    pub password: Option<String>,
}

impl Config {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self> {
        // Load .env file if it exists (ignore if missing)
        dotenvy::dotenv().ok();

        // Build database URL from components or use direct URL
        let database_url = Self::build_database_url()?;

        Self::validate_database_url(&database_url)?;

        // Load and validate API_KEY
        let api_key = env::var("API_KEY").context("API_KEY environment variable is required")?;

        if api_key.len() < 32 {
            return Err(anyhow!(
                "API_KEY must be at least 32 characters long (current length: {})",
                api_key.len()
            ));
        }

        // Load optional PORT with default 3000
        let port = env::var("PORT")
            .ok()
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(8080);

        // Load optional RUST_LOG with default "info"
        let rust_log = env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());

        // Build NATS config from components (optional)
        let nats_config = Self::build_nats_config()?;

        Ok(Config {
            database_url,
            api_key,
            port,
            rust_log,
            nats_config,
        })
    }

    /// Validate database URL format
    fn validate_database_url(url: &str) -> Result<()> {
        if !url.starts_with("postgresql://") && !url.starts_with("postgres://") {
            return Err(anyhow!(
                "DATABASE_URL must start with 'postgresql://' or 'postgres://' (got: {})",
                url
            ));
        }
        Ok(())
    }

    /// Build database URL from environment variables
    ///
    /// Uses component-based configuration:
    /// PG_HOST, PG_PORT, PG_USER, PG_PASSWORD, PG_DB_NAME
    ///
    /// Returns error if required variables are not set
    fn build_database_url() -> Result<String> {
        // Component-based configuration (required)
        let pg_host = env::var("PG_HOST").context("PG_HOST is required")?;

        let pg_port = env::var("PG_PORT")
            .ok()
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(5432); // Default PostgreSQL port

        let pg_user = env::var("PG_USER")
            .context("PG_USER is required")?;

        let pg_password = env::var("PG_PASSWORD")
            .context("PG_PASSWORD is required")?;

        let pg_db_name = env::var("PG_DB_NAME")
            .context("PG_DB_NAME is required")?;

        // URL encode credentials to handle special characters
        let encoded_user = urlencoding::encode(&pg_user);
        let encoded_password = urlencoding::encode(&pg_password);

        // Build PostgreSQL connection URL
        let url = format!(
            "postgresql://{}:{}@{}:{}/{}",
            encoded_user, encoded_password, pg_host, pg_port, pg_db_name
        );

        Ok(url)
    }

    /// Build NATS configuration from environment variables
    ///
    /// Uses component-based configuration:
    /// NATS_HOST, NATS_PORT, NATS_USER, NATS_PASSWORD
    ///
    /// Returns None if NATS is not configured (NATS_HOST not set)
    fn build_nats_config() -> Result<Option<NatsConfig>> {
        // Component-based configuration (optional)
        let nats_host = env::var("NATS_HOST").ok();

        // If no host is specified, NATS is not configured
        if nats_host.is_none() {
            return Ok(None);
        }

        let host = nats_host.unwrap();
        let port = env::var("NATS_PORT")
            .ok()
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(4222); // Default NATS port

        // Get user and password, treating empty strings as None
        let user = env::var("NATS_USER")
            .ok()
            .and_then(|s| if s.trim().is_empty() { None } else { Some(s) });

        let password = env::var("NATS_PASSWORD")
            .ok()
            .and_then(|s| if s.trim().is_empty() { None } else { Some(s) });

        Ok(Some(NatsConfig {
            host,
            port,
            user,
            password,
        }))
    }
}
