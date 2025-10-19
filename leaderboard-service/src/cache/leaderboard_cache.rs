use redis::aio::ConnectionManager;
use redis::{AsyncCommands, RedisError};
use shared::errors::{Result, ServiceError};
use std::sync::Arc;
use tracing::{debug, error, info, warn};

const LEADERBOARD_KEY: &str = "leaderboard:global";
const USER_MEMBER_MAP_KEY: &str = "leaderboard:user_members";
const DEFAULT_LEADERBOARD_LIMIT: i32 = 20;

#[derive(Debug, Clone)]
pub struct LeaderboardEntry {
    pub rank: i32,
    pub user_id: String,
    pub username: String,
    pub total_clicks: i64,
}

#[derive(Clone)]
pub struct LeaderboardCache {
    redis: Arc<ConnectionManager>,
}

impl LeaderboardCache {
    pub fn new(redis: ConnectionManager) -> Self {
        Self {
            redis: Arc::new(redis),
        }
    }

    pub async fn update_score(
        &self,
        user_id: &str,
        username: &str,
        score: i64,
    ) -> Result<i32> {
        let member = format!("{}:{}", user_id, username);

        let mut conn = self.redis.as_ref().clone();

        conn.zadd(LEADERBOARD_KEY, &member, score)
            .await
            .map_err(|e: RedisError| {
                error!("Failed to update score for user {}: {}", user_id, e);
                ServiceError::Redis(e.to_string())
            })?;

        conn.hset(USER_MEMBER_MAP_KEY, user_id, &member)
            .await
            .map_err(|e: RedisError| {
                error!("Failed to update user member map for user {}: {}", user_id, e);
                ServiceError::Redis(e.to_string())
            })?;

        debug!("Updated score for user {} to {}", user_id, score);

        self.get_user_rank(user_id).await
    }


    pub async fn get_user_rank(&self, user_id: &str) -> Result<i32> {
        let mut conn = self.redis.as_ref().clone();

        let member_name: Option<String> = conn
            .hget(USER_MEMBER_MAP_KEY, user_id)
            .await
            .map_err(|e: RedisError| {
                error!("Failed to get member name for user {}: {}", user_id, e);
                ServiceError::Redis(e.to_string())
            })?;

        let member = match member_name {
            Some(m) => m,
            None => {
                debug!("User {} not found in leaderboard", user_id);
                return Ok(0);
            }
        };

        let rank: Option<i64> = conn
            .zrevrank(LEADERBOARD_KEY, &member)
            .await
            .map_err(|e: RedisError| {
                error!("Failed to get rank for user {}: {}", user_id, e);
                ServiceError::Redis(e.to_string())
            })?;

        Ok(rank.map(|r| (r + 1) as i32).unwrap_or(0))
    }

    pub async fn get_user_score(&self, user_id: &str) -> Result<Option<i64>> {
        let mut conn = self.redis.as_ref().clone();

        let member_name: Option<String> = conn
            .hget(USER_MEMBER_MAP_KEY, user_id)
            .await
            .map_err(|e: RedisError| {
                error!("Failed to get member name for user {}: {}", user_id, e);
                ServiceError::Redis(e.to_string())
            })?;

        let member = match member_name {
            Some(m) => m,
            None => {
                debug!("User {} not found in leaderboard", user_id);
                return Ok(None);
            }
        };

        let score: Option<i64> = conn
            .zscore(LEADERBOARD_KEY, &member)
            .await
            .map_err(|e: RedisError| {
                error!("Failed to get score for user {}: {}", user_id, e);
                ServiceError::Redis(e.to_string())
            })?;

        Ok(score)
    }

    pub async fn get_leaderboard(
        &self,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> Result<Vec<LeaderboardEntry>> {
        let limit = limit.unwrap_or(DEFAULT_LEADERBOARD_LIMIT);
        let offset = offset.unwrap_or(0);

        let mut conn = self.redis.as_ref().clone();

        let start = offset as isize;
        let end = (offset + limit - 1) as isize;

        let entries: Vec<(String, i64)> = conn
            .zrevrange_withscores(LEADERBOARD_KEY, start, end)
            .await
            .map_err(|e: RedisError| {
                error!("Failed to get leaderboard: {}", e);
                ServiceError::Redis(e.to_string())
            })?;

        let mut result = Vec::with_capacity(entries.len());
        for (rank_idx, (member, score)) in entries.iter().enumerate() {
            let parts: Vec<&str> = member.splitn(2, ':').collect();
            if parts.len() == 2 {
                result.push(LeaderboardEntry {
                    rank: (offset + rank_idx as i32 + 1),
                    user_id: parts[0].to_string(),
                    username: parts[1].to_string(),
                    total_clicks: *score,
                });
            } else {
                warn!("Invalid member format in leaderboard: {}", member);
            }
        }

        debug!("Retrieved {} leaderboard entries", result.len());
        Ok(result)
    }

    pub async fn get_total_count(&self) -> Result<i64> {
        let mut conn = self.redis.as_ref().clone();

        let count: i64 = conn
            .zcard(LEADERBOARD_KEY)
            .await
            .map_err(|e: RedisError| {
                error!("Failed to get leaderboard count: {}", e);
                ServiceError::Redis(e.to_string())
            })?;

        Ok(count)
    }

    pub async fn remove_user(&self, user_id: &str) -> Result<bool> {
        let mut conn = self.redis.as_ref().clone();

        let members: Vec<String> = conn
            .zrevrange(LEADERBOARD_KEY, 0, -1)
            .await
            .map_err(|e: RedisError| {
                error!("Failed to get members for removal: {}", e);
                ServiceError::Redis(e.to_string())
            })?;

        for member in members {
            if member.starts_with(&format!("{}:", user_id)) {
                let removed: i32 = conn
                    .zrem(LEADERBOARD_KEY, &member)
                    .await
                    .map_err(|e: RedisError| {
                        error!("Failed to remove user {}: {}", user_id, e);
                        ServiceError::Redis(e.to_string())
                    })?;

                info!("Removed user {} from leaderboard", user_id);
                return Ok(removed > 0);
            }
        }

        Ok(false)
    }

    pub async fn clear(&self) -> Result<()> {
        let mut conn = self.redis.as_ref().clone();

        conn.del(LEADERBOARD_KEY)
            .await
            .map_err(|e: RedisError| {
                error!("Failed to clear leaderboard: {}", e);
                ServiceError::Redis(e.to_string())
            })?;

        warn!("Leaderboard cleared");
        Ok(())
    }
}
