
use redis::aio::MultiplexedConnection;
use redis::AsyncCommands;
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, info, warn};

use shared::{Result, ServiceError};
use crate::repository::UserRepository;
use crate::stream::ClickEventPublisher;

const REDIS_CLICKS_PREFIX: &str = "clicks:pending:shard:";
const REDIS_USERNAMES_KEY: &str = "clicks:usernames";

const MAX_BATCH_SIZE: usize = 20;

pub struct RedisClickAccumulator {
    redis: MultiplexedConnection,
    user_repo: UserRepository,
    event_publisher: Option<ClickEventPublisher>,
    flush_interval: Duration,
    shard_id: usize,
    num_shards: usize,
}

impl RedisClickAccumulator {

    pub fn new(
        redis: MultiplexedConnection,
        user_repo: UserRepository,
        event_publisher: Option<ClickEventPublisher>,
        flush_interval_ms: u64,
        shard_id: usize,
        num_shards: usize,
    ) -> Self {
        Self {
            redis,
            user_repo,
            event_publisher,
            flush_interval: Duration::from_millis(flush_interval_ms),
            shard_id,
            num_shards,
        }
    }


    pub async fn accumulate_click(
        &self,
        user_id: &str,
        username: &str,
        count: u32,
    ) -> Result<u32> {
        let mut redis = self.redis.clone();

        let clicks_key = format!("{}{}", REDIS_CLICKS_PREFIX, self.shard_id);

        let new_count: u32 = redis
            .hincr(&clicks_key, user_id, count)
            .await
            .map_err(|e| {
                error!(error = %e, count = count, "Failed to increment click count in Redis");
                ServiceError::Internal(format!("Redis HINCRBY failed: {}", e))
            })?;

        let _: () = redis
            .hset(REDIS_USERNAMES_KEY, user_id, username)
            .await
            .map_err(|e| {
                warn!(error = %e, "Failed to cache username in Redis");
                e
            })
            .unwrap_or(());

        debug!(
            user_id = %user_id,
            count = count,
            accumulated = new_count,
            "Click(s) accumulated in Redis"
        );

        Ok(new_count)
    }

    pub async fn flush_batch(&mut self) -> Result<usize> {
        let clicks_key = format!("{}{}", REDIS_CLICKS_PREFIX, self.shard_id);

        let mut pending_clicks: HashMap<String, i64> = self
            .redis
            .hgetall(&clicks_key)
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to fetch pending clicks from Redis");
                ServiceError::Internal(format!("Redis HGETALL failed: {}", e))
            })?;

        let user_count = if pending_clicks.is_empty() {
            debug!(shard_id = self.shard_id, "No user clicks to flush for this shard");
            0
        } else {
            let batch_size = pending_clicks.len();

            let _: () = self
                .redis
                .del(&clicks_key)
                .await
                .map_err(|e| {
                    warn!(error = %e, "Failed to clear Redis clicks after fetch");
                    e
                })
                .unwrap_or(());

            let user_ids: Vec<&String> = pending_clicks.keys().collect();
            let usernames: HashMap<String, String> = self
                .redis
                .hget(REDIS_USERNAMES_KEY, &user_ids)
                .await
                .unwrap_or_default();

            let total_clicks: i64 = pending_clicks.values().sum();

            info!(
                shard_id = self.shard_id,
                users = batch_size,
                total_clicks = total_clicks,
                "Flushing Redis user click batch to database"
            );

            let pending_clicks: HashMap<String, i64> = if pending_clicks.len() > MAX_BATCH_SIZE {
                warn!(
                    total = pending_clicks.len(),
                    limit = MAX_BATCH_SIZE,
                    "Batch size exceeded limit, processing first {} users",
                    MAX_BATCH_SIZE
                );
                pending_clicks.into_iter().take(MAX_BATCH_SIZE).collect()
            } else {
                pending_clicks
            };

            let batches: HashMap<String, super::click_batch_accumulator::UserClickBatch> =
                pending_clicks
                    .into_iter()
                    .map(|(user_id, count)| {
                        let username = usernames
                            .get(&user_id)
                            .cloned()
                            .unwrap_or_else(|| "Unknown".to_string());

                        (
                            user_id.clone(),
                            super::click_batch_accumulator::UserClickBatch {
                                username,
                                accumulated_clicks: count as u32,
                                last_click_time: chrono::Utc::now(),
                            },
                        )
                    })
                    .collect();

            let updated_totals = self.bulk_update_with_retry(&batches).await?;

            if let Some(publisher) = &self.event_publisher {
                self.publish_batch_events(publisher, &batches, &updated_totals).await;
            }

            info!(
                users = batch_size,
                total_clicks = total_clicks,
                "Redis user click batch flushed successfully"
            );

            batch_size
        };

        Ok(user_count)
    }

    async fn bulk_update_with_retry(
        &self,
        batches: &HashMap<String, super::click_batch_accumulator::UserClickBatch>,
    ) -> Result<HashMap<String, i64>> {
        const MAX_RETRIES: u32 = 3;
        let mut attempt = 0;

        loop {
            match self.user_repo.bulk_increment_clicks(batches).await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    let err_msg = e.to_string();
                    if err_msg.contains("deadlock") && attempt < MAX_RETRIES {
                        attempt += 1;
                        let delay_ms = 50 * (1 << attempt); // 100ms, 200ms, 400ms
                        warn!(
                            attempt = attempt,
                            delay_ms = delay_ms,
                            "Deadlock detected, retrying after delay"
                        );
                        tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                    } else {
                        return Err(e);
                    }
                }
            }
        }
    }

    async fn publish_batch_events(
        &self,
        publisher: &ClickEventPublisher,
        batches: &HashMap<String, super::click_batch_accumulator::UserClickBatch>,
        updated_totals: &HashMap<String, i64>,
    ) {
        for (user_id, batch) in batches.iter() {
            let total_clicks = updated_totals.get(user_id).copied().unwrap_or_else(|| {
                warn!(
                    user_id = %user_id,
                    "User not found in updated totals, using batch count as fallback"
                );
                batch.accumulated_clicks as i64
            });

            let publisher_clone = publisher.clone();
            let user_id = user_id.clone();
            let username = batch.username.clone();

            tokio::spawn(async move {
                if let Err(e) = publisher_clone
                    .publish_click_event(&user_id, &username, total_clicks)
                    .await
                {
                    error!(
                        user_id = %user_id,
                        error = %e,
                        "Failed to publish batch click event to stream"
                    );
                }
            });
        }

        debug!(
            events = batches.len(),
            "Published click events to Redis Streams with total clicks"
        );
    }

    pub fn start_background_flusher(self: Arc<Self>) {
        let interval = self.flush_interval;

        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);

            info!(
                interval_ms = interval.as_millis(),
                "Started background Redis click batch flusher"
            );

            loop {
                ticker.tick().await;

                let redis = self.redis.clone();
                let user_repo = self.user_repo.clone();
                let event_publisher = self.event_publisher.clone();
                let flush_interval = self.flush_interval;

                let mut accumulator = RedisClickAccumulator::new(
                    redis,
                    user_repo,
                    event_publisher,
                    flush_interval.as_millis() as u64,
                    self.shard_id,
                    self.num_shards,
                );

                match accumulator.flush_batch().await {
                    Ok(count) if count > 0 => {
                        debug!(users = count, "Redis batch flush cycle completed");
                    }
                    Ok(_) => {
                    }
                    Err(e) => {
                        error!(error = %e, "Redis batch flush cycle failed");
                        // Continue running - don't crash on error
                    }
                }
            }
        });
    }
}

impl Clone for RedisClickAccumulator {
    fn clone(&self) -> Self {
        Self {
            redis: self.redis.clone(),
            user_repo: self.user_repo.clone(),
            event_publisher: self.event_publisher.clone(),
            flush_interval: self.flush_interval,
            shard_id: self.shard_id,
            num_shards: self.num_shards,
        }
    }
}


fn get_shard_for_user(user_id: &str, num_shards: usize) -> usize {
    let mut hasher = DefaultHasher::new();
    user_id.hash(&mut hasher);
    (hasher.finish() as usize) % num_shards
}
