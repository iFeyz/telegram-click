

use redis::aio::MultiplexedConnection;
use redis::{AsyncCommands, RedisError};
use shared::errors::{Result, ServiceError};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error};

const STREAM_KEY: &str = "clicks:stream";

#[derive(Clone)]
pub struct ClickEventPublisher {
    redis: Arc<Mutex<MultiplexedConnection>>,
}

impl ClickEventPublisher {
    pub fn new(redis: MultiplexedConnection) -> Self {
        Self {
            redis: Arc::new(Mutex::new(redis)),
        }
    }

    pub async fn publish_click_event(
        &self,
        user_id: &str,
        username: &str,
        total_clicks: i64,
    ) -> Result<String> {
        let mut conn = self.redis.lock().await;
        let timestamp = chrono::Utc::now().timestamp();

        debug!(
            "Publishing click event: user_id={}, username={}, total_clicks={}",
            user_id, username, total_clicks
        );

        let message_id: String = conn
            .xadd(
                STREAM_KEY,
                "*",
                &[
                    ("user_id", user_id),
                    ("username", username),
                    ("total_clicks", &total_clicks.to_string()),
                    ("timestamp", &timestamp.to_string()),
                ],
            )
            .await
            .map_err(|e: RedisError| {
                error!("Failed to publish click event: {}", e);
                ServiceError::Redis(e.to_string())
            })?;

        debug!("Published click event with message_id: {}", message_id);

        Ok(message_id)
    }

    pub async fn health_check(&self) -> bool {
        let mut conn = self.redis.lock().await;
        let result: std::result::Result<String, RedisError> = redis::cmd("PING")
            .query_async(&mut *conn)
            .await;

        match result {
            Ok(_) => true,
            Err(e) => {
                error!("Redis health check failed: {}", e);
                false
            }
        }
    }
}
