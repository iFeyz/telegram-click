use shared::{Result, UserId, SessionId};
use crate::domain::RateLimiter;
use crate::repository::{UserRepository, SessionRepository};
use crate::service::RedisClickAccumulator;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct ClickResult {
    pub total_clicks: i64,
}


pub struct ClickService {
    user_repo: UserRepository,
    session_repo: SessionRepository,
    rate_limiter: Arc<tokio::sync::Mutex<RateLimiter>>,
    batch_accumulator: Arc<RedisClickAccumulator>,
}

impl ClickService {

    pub fn new(
        user_repo: UserRepository,
        session_repo: SessionRepository,
        rate_limiter: Arc<tokio::sync::Mutex<RateLimiter>>,
        batch_accumulator: Arc<RedisClickAccumulator>,
    ) -> Self {
        Self {
            user_repo,
            session_repo,
            rate_limiter,
            batch_accumulator,
        }
    }


    #[tracing::instrument(skip(self), fields(user_id = %user_id, username = %username, click_count = click_count))]
    pub async fn process_click(
        &self,
        user_id: &UserId,
        username: &str,
        session_id: &SessionId,
        click_count: u32,
    ) -> Result<ClickResult> {
        let total_start = std::time::Instant::now();

        shared::record_counter("game_service.click.requests", 1);

        let rate_limit_start = std::time::Instant::now();
        let mut rate_limiter = self.rate_limiter.lock().await;
        let rate_limit_lock_time = rate_limit_start.elapsed();
        shared::record_timing("game_service.rate_limit.lock_wait", rate_limit_lock_time.as_secs_f64());

        let rate_check_start = std::time::Instant::now();
        match rate_limiter.check_rate_limit(user_id, click_count).await {
            Ok(_) => {
                let rate_check_time = rate_check_start.elapsed();
                shared::record_timing("game_service.rate_limit.check", rate_check_time.as_secs_f64());
                drop(rate_limiter); // Release lock immediately

                tracing::debug!(
                    lock_ms = rate_limit_lock_time.as_millis(),
                    check_ms = rate_check_time.as_millis(),
                    click_count = click_count,
                    "Rate limit check passed for batch"
                );
            }
            Err(e) => {
                shared::record_counter("game_service.click.rate_limited", 1);
                drop(rate_limiter);
                tracing::warn!(
                    click_count = click_count,
                    "Rate limit exceeded for batch"
                );
                return Err(e);
            }
        }

        let accumulate_start = std::time::Instant::now();
        let pending_count = self.batch_accumulator
            .accumulate_click(&user_id.to_string(), username, click_count)
            .await?;
        let accumulate_time = accumulate_start.elapsed();
        shared::record_timing("game_service.click.accumulate", accumulate_time.as_secs_f64());

        let user_fetch_start = std::time::Instant::now();
        let user = self.user_repo.get_by_id(user_id).await?;
        let user_fetch_time = user_fetch_start.elapsed();
        shared::record_timing("game_service.user.get_by_id", user_fetch_time.as_secs_f64());

        let estimated_total = user.total_clicks + pending_count as i64;

        let total_time = total_start.elapsed();
        shared::record_timing("game_service.click.total_latency", total_time.as_secs_f64());
        shared::record_counter("game_service.click.success", 1);

        tracing::info!(
            total_ms = total_time.as_millis(),
            rate_limit_ms = (rate_limit_lock_time + rate_check_start.elapsed()).as_millis(),
            accumulate_ms = accumulate_time.as_millis(),
            user_fetch_ms = user_fetch_time.as_millis(),
            pending = pending_count,
            db_total = user.total_clicks,
            estimated_total = estimated_total,
            "Click processed successfully"
        );

        Ok(ClickResult {
            total_clicks: estimated_total,
        })
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::ServiceError;

    #[test]
    fn test_click_error_types() {
        let error = ServiceError::RateLimitExceeded;
        assert!(matches!(error, ServiceError::RateLimitExceeded));

        let user_id = UserId::new();
        let error = ServiceError::UserNotFound(user_id.to_string());
        assert!(matches!(error, ServiceError::UserNotFound(_)));
    }

}
