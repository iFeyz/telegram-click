use crate::cache::{LeaderboardCache, StatsCache};
use redis::aio::ConnectionManager;
use redis::RedisError;
use shared::errors::{Result, ServiceError};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

const STREAM_KEY: &str = "clicks:stream";
const CONSUMER_GROUP: &str = "leaderboard-service";
const CONSUMER_NAME: &str = "leaderboard-consumer-1";
const BATCH_SIZE: usize = 100;
const BLOCK_MS: usize = 5000;
#[derive(Debug, Clone)]
pub struct ClickEvent {
    pub user_id: String,
    pub username: String,
    pub total_clicks: i64,
    pub timestamp: i64,
}

#[derive(Clone)]
pub struct ClickStreamConsumer {
    redis: Arc<ConnectionManager>,
    leaderboard_cache: Arc<LeaderboardCache>,
    stats_cache: Arc<StatsCache>,
}

impl ClickStreamConsumer {
    pub fn new(
        redis: ConnectionManager,
        leaderboard_cache: LeaderboardCache,
        stats_cache: StatsCache,
    ) -> Self {
        Self {
            redis: Arc::new(redis),
            leaderboard_cache: Arc::new(leaderboard_cache),
            stats_cache: Arc::new(stats_cache),
        }
    }

    pub async fn init_consumer_group(&self) -> Result<()> {
        let mut conn = self.redis.as_ref().clone();

        let result: std::result::Result<String, RedisError> = redis::cmd("XGROUP")
            .arg("CREATE")
            .arg(STREAM_KEY)
            .arg(CONSUMER_GROUP)
            .arg("$")
            .arg("MKSTREAM")
            .query_async(&mut conn)
            .await;

        match result {
            Ok(_) => {
                info!("Created consumer group: {}", CONSUMER_GROUP);
                Ok(())
            }
            Err(e) => {
                let err_msg = e.to_string();
                if err_msg.contains("BUSYGROUP") {
                    info!("Consumer group already exists: {}", CONSUMER_GROUP);
                    Ok(())
                } else {
                    error!("Failed to create consumer group: {}", e);
                    Err(ServiceError::Redis(e.to_string()))
                }
            }
        }
    }

    pub async fn start_consuming(&self) -> Result<()> {
        info!("Starting click stream consumer");

        loop {
            match self.consume_batch().await {
                Ok(count) => {
                    if count > 0 {
                        debug!("Processed {} click events", count);
                    }
                }
                Err(e) => {
                    error!("Error consuming stream batch: {}", e);
                    sleep(Duration::from_secs(5)).await;
                }
            }
        }
    }

    async fn consume_batch(&self) -> Result<usize> {
        let mut conn = self.redis.as_ref().clone();

        let result: std::result::Result<redis::Value, RedisError> =
            redis::cmd("XREADGROUP")
                .arg("GROUP")
                .arg(CONSUMER_GROUP)
                .arg(CONSUMER_NAME)
                .arg("COUNT")
                .arg(BATCH_SIZE)
                .arg("BLOCK")
                .arg(BLOCK_MS)
                .arg("STREAMS")
                .arg(STREAM_KEY)
                .arg(">")
                .query_async(&mut conn)
                .await;

        match result {
            Ok(redis::Value::Nil) => {
                Ok(0)
            }
            Ok(redis::Value::Array(streams)) => {
                let mut processed = 0;

                for stream in streams {
                    if let redis::Value::Array(stream_data) = stream {
                        if stream_data.len() >= 2 {
                            if let redis::Value::Array(entries) = &stream_data[1] {
                                for entry in entries {
                                    if let redis::Value::Array(entry_data) = entry {
                                        if entry_data.len() >= 2 {
                                            let message_id = match &entry_data[0] {
                                                redis::Value::BulkString(bytes) => {
                                                    String::from_utf8_lossy(&bytes).to_string()
                                                }
                                                _ => continue,
                                            };

                                            if let redis::Value::Array(fields_array) = &entry_data[1] {
                                                match self.parse_and_process_event(fields_array).await {
                                                    Ok(_) => {
                                                        let _: std::result::Result<i32, RedisError> =
                                                            redis::cmd("XACK")
                                                                .arg(STREAM_KEY)
                                                                .arg(CONSUMER_GROUP)
                                                                .arg(&message_id)
                                                                .query_async(&mut conn)
                                                                .await;

                                                        processed += 1;
                                                    }
                                                    Err(e) => {
                                                        error!(
                                                            "Failed to process event {}: {}",
                                                            message_id, e
                                                        );
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                Ok(processed)
            }
            Ok(_) => {
                warn!("Unexpected Redis response format");
                Ok(0)
            }
            Err(e) => Err(ServiceError::Redis(e.to_string())),
        }
    }

    async fn parse_and_process_event(&self, fields_array: &[redis::Value]) -> Result<()> {
        let mut fields = HashMap::new();

        for chunk in fields_array.chunks(2) {
            if chunk.len() == 2 {
                let key = match &chunk[0] {
                    redis::Value::BulkString(bytes) => String::from_utf8_lossy(&bytes).to_string(),
                    _ => continue,
                };

                let value = match &chunk[1] {
                    redis::Value::BulkString(bytes) => String::from_utf8_lossy(&bytes).to_string(),
                    _ => continue,
                };

                fields.insert(key, value);
            }
        }

        self.process_event(&fields).await
    }

    async fn process_event(&self, fields: &HashMap<String, String>) -> Result<()> {
        let user_id = fields
            .get("user_id")
            .ok_or_else(|| ServiceError::Validation("Missing user_id field".to_string()))?;

        let username = fields
            .get("username")
            .ok_or_else(|| ServiceError::Validation("Missing username field".to_string()))?;

        let total_clicks = fields
            .get("total_clicks")
            .and_then(|s| s.parse::<i64>().ok())
            .ok_or_else(|| ServiceError::Validation("Invalid total_clicks field".to_string()))?;

        let _timestamp = fields
            .get("timestamp")
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(0);

        debug!(
            "Processing click event: user={}, username={}, clicks={}",
            user_id, username, total_clicks
        );

        let new_rank = self
            .leaderboard_cache
            .update_score(user_id, username, total_clicks)
            .await?;

        debug!(
            "Updated leaderboard: user={}, new_rank={}",
            user_id, new_rank
        );

        self.stats_cache.increment_total_clicks(1).await?;

        Ok(())
    }

    pub async fn get_pending_count(&self) -> Result<usize> {
        let mut conn = self.redis.as_ref().clone();

        let result: Vec<redis::Value> = redis::cmd("XPENDING")
            .arg(STREAM_KEY)
            .arg(CONSUMER_GROUP)
            .query_async(&mut conn)
            .await
            .map_err(|e: RedisError| ServiceError::Redis(e.to_string()))?;

        if let Some(redis::Value::Int(count)) = result.first() {
            Ok(*count as usize)
        } else {
            Ok(0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_init_consumer_group() {
        let client = redis::Client::open("redis://127.0.0.1:6380").unwrap();
        let conn = ConnectionManager::new(client).await.unwrap();

        let leaderboard_cache = LeaderboardCache::new(conn.clone());
        let stats_cache = StatsCache::new(conn.clone());
        let consumer = ClickStreamConsumer::new(conn, leaderboard_cache, stats_cache);

        let result = consumer.init_consumer_group().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore]
    async fn test_process_event() {
        let client = redis::Client::open("redis://127.0.0.1:6380").unwrap();
        let conn = ConnectionManager::new(client).await.unwrap();

        let leaderboard_cache = LeaderboardCache::new(conn.clone());
        let stats_cache = StatsCache::new(conn.clone());
        let consumer = ClickStreamConsumer::new(conn, leaderboard_cache, stats_cache);

        let mut fields = HashMap::new();
        fields.insert("user_id".to_string(), "test-user".to_string());
        fields.insert("username".to_string(), "TestUser".to_string());
        fields.insert("total_clicks".to_string(), "42".to_string());
        fields.insert("timestamp".to_string(), "1234567890".to_string());

        let result = consumer.process_event(&fields).await;
        assert!(result.is_ok());

        let rank = consumer
            .leaderboard_cache
            .get_user_rank("test-user")
            .await
            .unwrap();
        assert!(rank > 0);
    }
}
