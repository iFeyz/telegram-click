use redis::aio::MultiplexedConnection;
use redis::AsyncCommands;
use shared::{Result, ServiceError, UserId};

pub struct RateLimiter {
    redis: MultiplexedConnection,
    max_clicks_per_second: u32,
}

impl RateLimiter {

    pub fn new(redis: MultiplexedConnection, max_clicks_per_second: u32) -> Self {
        Self {
            redis,
            max_clicks_per_second,
        }
    }

    pub async fn check_rate_limit(&mut self, user_id: &UserId, click_count: u32) -> Result<()> {
        // Create key with 1-second window
        let key = format!("rate_limit:{}", user_id);

        let count: u32 = self
            .redis
            .incr(&key, click_count)
            .await
            .map_err(|e| ServiceError::Redis(e.to_string()))?;

        if count == click_count {
            self.redis
                .expire(&key, 1)
                .await
                .map_err(|e| ServiceError::Redis(e.to_string()))?;
        }

        if count > self.max_clicks_per_second {
            return Err(ServiceError::RateLimitExceeded);
        }

        Ok(())
    }

    pub async fn get_current_count(&mut self, user_id: &UserId) -> Result<u32> {
        let key = format!("rate_limit:{}", user_id);

        let count: Option<u32> = self
            .redis
            .get(&key)
            .await
            .map_err(|e| ServiceError::Redis(e.to_string()))?;

        Ok(count.unwrap_or(0))
    }

    pub async fn reset(&mut self, user_id: &UserId) -> Result<()> {
        let key = format!("rate_limit:{}", user_id);

        self.redis
            .del(&key)
            .await
            .map_err(|e| ServiceError::Redis(e.to_string()))?;

        Ok(())
    }
}
