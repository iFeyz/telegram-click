use std::env;

use crate::errors::{Result, ServiceError};

#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
}

impl DatabaseConfig {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            url: env::var("DATABASE_URL")
                .map_err(|_| ServiceError::Internal("DATABASE_URL not set".to_string()))?,
            max_connections: env::var("MAX_CONNECTIONS")
                .unwrap_or_else(|_| "100".to_string())
                .parse()
                .map_err(|e| ServiceError::Internal(format!("Invalid MAX_CONNECTIONS: {}", e)))?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct RedisConfig {
    pub url: String,
}

impl RedisConfig {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            url: env::var("REDIS_URL")
                .map_err(|_| ServiceError::Internal("REDIS_URL not set".to_string()))?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ServiceConfig {
    pub port: u16,
    pub click_rate_limit: u32,
    pub session_timeout_secs: i64,
}

impl ServiceConfig {
    pub fn from_env(default_port: u16) -> Result<Self> {
        Ok(Self {
            port: default_port,
            click_rate_limit: env::var("CLICK_RATE_LIMIT")
                .unwrap_or_else(|_| "10".to_string())
                .parse()
                .map_err(|e| ServiceError::Internal(format!("Invalid CLICK_RATE_LIMIT: {}", e)))?,
            session_timeout_secs: env::var("SESSION_TIMEOUT_SECS")
                .unwrap_or_else(|_| "300".to_string())
                .parse()
                .map_err(|e| {
                    ServiceError::Internal(format!("Invalid SESSION_TIMEOUT_SECS: {}", e))
                })?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct BatchConfig {
    pub click_flush_interval_ms: u64,
    pub leaderboard_broadcast_interval_ms: u64,
}

impl BatchConfig {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            click_flush_interval_ms: env::var("CLICK_BATCH_FLUSH_INTERVAL_MS")
                .unwrap_or_else(|_| "1000".to_string())
                .parse()
                .map_err(|e| {
                    ServiceError::Internal(format!("Invalid CLICK_BATCH_FLUSH_INTERVAL_MS: {}", e))
                })?,
            leaderboard_broadcast_interval_ms: env::var("LEADERBOARD_BROADCAST_INTERVAL_MS")
                .unwrap_or_else(|_| "1000".to_string())
                .parse()
                .map_err(|e| {
                    ServiceError::Internal(format!(
                        "Invalid LEADERBOARD_BROADCAST_INTERVAL_MS: {}", e
                    ))
                })?,
        })
    }
}
