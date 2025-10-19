use redis::aio::ConnectionManager;
use redis::{AsyncCommands, RedisError};
use shared::errors::{Result, ServiceError};
use std::sync::Arc;
use tracing::{debug, error};

const TOTAL_CLICKS_KEY: &str = "stats:total_clicks";
const TOTAL_USERS_KEY: &str = "stats:total_users";
const ACTIVE_SESSIONS_KEY: &str = "stats:active_sessions";

#[derive(Debug, Clone, Default)]
pub struct GlobalStats {
    pub total_clicks: i64,
    pub total_users: i64,
    pub active_sessions: i64,
}

#[derive(Clone)]
pub struct StatsCache {
    redis: Arc<ConnectionManager>,
}

impl StatsCache {
    pub fn new(redis: ConnectionManager) -> Self {
        Self {
            redis: Arc::new(redis),
        }
    }

    pub async fn get_global_stats(&self) -> Result<GlobalStats> {
        let mut conn = self.redis.as_ref().clone();

        let values: Vec<Option<i64>> = redis::cmd("MGET")
            .arg(TOTAL_CLICKS_KEY)
            .arg(TOTAL_USERS_KEY)
            .arg(ACTIVE_SESSIONS_KEY)
            .query_async(&mut conn)
            .await
            .map_err(|e: RedisError| {
                error!("Failed to get global stats: {}", e);
                ServiceError::Redis(e.to_string())
            })?;

        let stats = GlobalStats {
            total_clicks: values.get(0).and_then(|v| *v).unwrap_or(0),
            total_users: values.get(1).and_then(|v| *v).unwrap_or(0),
            active_sessions: values.get(2).and_then(|v| *v).unwrap_or(0),
        };

        debug!("Retrieved global stats: {:?}", stats);
        Ok(stats)
    }

    pub async fn get_total_clicks(&self) -> Result<i64> {
        let mut conn = self.redis.as_ref().clone();

        let clicks: Option<i64> = conn
            .get(TOTAL_CLICKS_KEY)
            .await
            .map_err(|e: RedisError| {
                error!("Failed to get total clicks: {}", e);
                ServiceError::Redis(e.to_string())
            })?;

        Ok(clicks.unwrap_or(0))
    }

    pub async fn get_total_users(&self) -> Result<i64> {
        let mut conn = self.redis.as_ref().clone();

        let users: Option<i64> = conn
            .get(TOTAL_USERS_KEY)
            .await
            .map_err(|e: RedisError| {
                error!("Failed to get total users: {}", e);
                ServiceError::Redis(e.to_string())
            })?;

        Ok(users.unwrap_or(0))
    }

    pub async fn get_active_sessions(&self) -> Result<i64> {
        let mut conn = self.redis.as_ref().clone();

        let sessions: Option<i64> = conn
            .get(ACTIVE_SESSIONS_KEY)
            .await
            .map_err(|e: RedisError| {
                error!("Failed to get active sessions: {}", e);
                ServiceError::Redis(e.to_string())
            })?;

        Ok(sessions.unwrap_or(0))
    }

    pub async fn increment_total_clicks(&self, amount: i64) -> Result<i64> {
        let mut conn = self.redis.as_ref().clone();

        let new_total: i64 = conn
            .incr(TOTAL_CLICKS_KEY, amount)
            .await
            .map_err(|e: RedisError| {
                error!("Failed to increment total clicks: {}", e);
                ServiceError::Redis(e.to_string())
            })?;

        debug!("Incremented total clicks by {}, new total: {}", amount, new_total);
        Ok(new_total)
    }

    pub async fn increment_total_users(&self) -> Result<i64> {
        let mut conn = self.redis.as_ref().clone();

        let new_total: i64 = conn
            .incr(TOTAL_USERS_KEY, 1)
            .await
            .map_err(|e: RedisError| {
                error!("Failed to increment total users: {}", e);
                ServiceError::Redis(e.to_string())
            })?;

        debug!("Incremented total users, new total: {}", new_total);
        Ok(new_total)
    }

    pub async fn set_active_sessions(&self, count: i64) -> Result<()> {
        let mut conn = self.redis.as_ref().clone();

        conn.set::<_, _, ()>(ACTIVE_SESSIONS_KEY, count)
            .await
            .map_err(|e: RedisError| {
                error!("Failed to set active sessions: {}", e);
                ServiceError::Redis(e.to_string())
            })?;

        debug!("Set active sessions to {}", count);
        Ok(())
    }

    pub async fn reset_all(&self) -> Result<()> {
        let mut conn = self.redis.as_ref().clone();

        conn.del::<_, ()>(&[TOTAL_CLICKS_KEY, TOTAL_USERS_KEY, ACTIVE_SESSIONS_KEY])
            .await
            .map_err(|e: RedisError| {
                error!("Failed to reset statistics: {}", e);
                ServiceError::Redis(e.to_string())
            })?;

        debug!("Reset all statistics");
        Ok(())
    }
}