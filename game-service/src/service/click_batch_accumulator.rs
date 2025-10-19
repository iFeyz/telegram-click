

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use chrono::Utc;
use tracing::{debug, error, info, warn};
use futures::future::join_all;

use shared::{Result, ServiceError};
use crate::repository::UserRepository;
use crate::stream::ClickEventPublisher;

pub struct ClickBatchAccumulator {
    pending_clicks: Arc<RwLock<HashMap<String, UserClickBatch>>>,

    user_repo: UserRepository,

    event_publisher: Option<ClickEventPublisher>,

    flush_interval: Duration,
}

#[derive(Debug, Clone)]
pub struct UserClickBatch {
    pub username: String,
    pub accumulated_clicks: u32,
    pub last_click_time: chrono::DateTime<Utc>,
}

impl ClickBatchAccumulator {

    pub fn new(
        user_repo: UserRepository,
        event_publisher: Option<ClickEventPublisher>,
        flush_interval_ms: u64,
    ) -> Self {
        Self {
            pending_clicks: Arc::new(RwLock::new(HashMap::new())),
            user_repo,
            event_publisher,
            flush_interval: Duration::from_millis(flush_interval_ms),
        }
    }

    pub async fn accumulate_click(
        &self,
        user_id: &str,
        username: &str,
    ) -> Result<u32> {
        let mut pending = self.pending_clicks.write().await;

        let batch = pending
            .entry(user_id.to_string())
            .and_modify(|batch| {
                batch.accumulated_clicks += 1;
                batch.last_click_time = Utc::now();
            })
            .or_insert(UserClickBatch {
                username: username.to_string(),
                accumulated_clicks: 1,
                last_click_time: Utc::now(),
            });

        debug!(
            user_id = %user_id,
            accumulated = batch.accumulated_clicks,
            "Click accumulated in memory"
        );

        Ok(batch.accumulated_clicks)
    }


    pub async fn flush_batch(&self) -> Result<usize> {
        let batches = {
            let mut pending = self.pending_clicks.write().await;
            std::mem::take(&mut *pending)
        };

        if batches.is_empty() {
            return Ok(0);
        }

        let batch_size = batches.len();
        let total_clicks: u32 = batches.values().map(|b| b.accumulated_clicks).sum();

        info!(
            users = batch_size,
            total_clicks = total_clicks,
            "Flushing click batch to database"
        );

        const MAX_CHUNK_SIZE: usize = 50;

        let updated_totals = if batch_size > MAX_CHUNK_SIZE {
            info!(
                batch_size = batch_size,
                chunk_size = MAX_CHUNK_SIZE,
                chunks = (batch_size + MAX_CHUNK_SIZE - 1) / MAX_CHUNK_SIZE,
                "Processing large batch with concurrent chunks"
            );

            let batch_vec: Vec<(String, UserClickBatch)> = batches.clone().into_iter().collect();

            let chunks: Vec<_> = batch_vec.chunks(MAX_CHUNK_SIZE).collect();

            let tasks: Vec<_> = chunks.into_iter().map(|chunk| {
                let chunk_map: HashMap<String, UserClickBatch> = chunk.iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();
                let user_repo = self.user_repo.clone();

                tokio::spawn(async move {
                    user_repo.bulk_increment_clicks(&chunk_map).await
                })
            }).collect();

            let results = join_all(tasks).await;

            let mut aggregated_totals = HashMap::new();
            for result in results {
                match result {
                    Ok(Ok(chunk_totals)) => {
                        aggregated_totals.extend(chunk_totals);
                    }
                    Ok(Err(e)) => {
                        error!(error = %e, "Chunk processing failed");
                        return Err(e);
                    }
                    Err(e) => {
                        error!(error = %e, "Task join failed");
                        return Err(ServiceError::Internal(format!("Task join error: {}", e)));
                    }
                }
            }

            info!(
                updated_users = aggregated_totals.len(),
                "Concurrent chunk processing completed"
            );

            aggregated_totals
        } else {
            match self.bulk_increment_clicks(&batches).await {
                Ok(totals) => {
                    debug!("Database bulk update successful");
                    totals
                }
                Err(e) => {
                    error!(error = %e, "Failed to flush clicks to database");
                    return Err(e);
                }
            }
        };

        if let Some(publisher) = &self.event_publisher {
            self.publish_batch_events(publisher, &batches, &updated_totals).await;
        }

        info!(
            users = batch_size,
            total_clicks = total_clicks,
            "Click batch flushed successfully"
        );

        Ok(batch_size)
    }

    async fn bulk_increment_clicks(
        &self,
        batches: &HashMap<String, UserClickBatch>,
    ) -> Result<HashMap<String, i64>> {
        self.user_repo.bulk_increment_clicks(batches).await
    }


    async fn publish_batch_events(
        &self,
        publisher: &ClickEventPublisher,
        batches: &HashMap<String, UserClickBatch>,
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
                "Started background click batch flusher"
            );

            loop {
                ticker.tick().await;

                match self.flush_batch().await {
                    Ok(count) if count > 0 => {
                        debug!(users = count, "Batch flush cycle completed");
                    }
                    Ok(_) => {
                    }
                    Err(e) => {
                        error!(error = %e, "Batch flush cycle failed");
                    }
                }
            }
        });
    }


    #[cfg(test)]
    pub async fn get_pending_count(&self, user_id: &str) -> u32 {
        let pending = self.pending_clicks.read().await;
        pending.get(user_id).map(|b| b.accumulated_clicks).unwrap_or(0)
    }
}

