use shared::errors::{Result, ServiceError};
use sqlx::PgPool;
use tracing::{debug, error};

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct LeaderboardEntry {
    pub rank: i64,
    pub user_id: String,
    pub username: String,
    pub total_clicks: i64,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct GlobalStats {
    pub total_clicks: i64,
    pub total_users: i64,
    pub active_sessions: i64,
}

#[derive(Clone)]
pub struct LeaderboardRepository {
    pool: PgPool,
}

impl LeaderboardRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn get_leaderboard(
        &self,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<LeaderboardEntry>> {
        let entries = sqlx::query_as::<_, LeaderboardEntry>(
            r#"
            SELECT
                rank,
                user_id,
                username,
                total_clicks
            FROM (
                SELECT
                    DENSE_RANK() OVER (ORDER BY total_clicks DESC) as rank,
                    id::text as user_id,
                    username,
                    total_clicks
                FROM users
                WHERE total_clicks > 0
            ) ranked
            ORDER BY rank
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            error!("Failed to fetch leaderboard: {}", e);
            ServiceError::Database(e.to_string())
        })?;

        debug!("Fetched {} leaderboard entries", entries.len());
        Ok(entries)
    }

    pub async fn get_user_rank(&self, user_id: &str) -> Result<Option<(i32, i64)>> {
        let user_uuid = uuid::Uuid::parse_str(user_id).map_err(|e| {
            error!("Invalid UUID: {}", e);
            ServiceError::Validation(format!("Invalid user_id: {}", e))
        })?;

        let result = sqlx::query_as::<_, (i64, i64)>(
            r#"
            SELECT
                rank,
                total_clicks
            FROM (
                SELECT
                    DENSE_RANK() OVER (ORDER BY total_clicks DESC) as rank,
                    id,
                    total_clicks
                FROM users
                WHERE total_clicks > 0
            ) ranked
            WHERE id = $1
            "#,
        )
        .bind(user_uuid)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            error!("Failed to get user rank: {}", e);
            ServiceError::Database(e.to_string())
        })?;

        Ok(result.map(|(rank, clicks)| (rank as i32, clicks)))
    }

    pub async fn get_total_count(&self) -> Result<i64> {
        let (count,): (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*)
            FROM users
            WHERE total_clicks > 0
            "#,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            error!("Failed to get total count: {}", e);
            ServiceError::Database(e.to_string())
        })?;

        Ok(count)
    }

    pub async fn update_score(&self, user_id: &str, username: &str, score: i64) -> Result<i32> {
        let user_uuid = uuid::Uuid::parse_str(user_id).map_err(|e| {
            error!("Invalid UUID: {}", e);
            ServiceError::Validation(format!("Invalid user_id: {}", e))
        })?;

        sqlx::query(
            r#"
            UPDATE users
            SET total_clicks = $1, username = $2, updated_at = NOW()
            WHERE id = $3
            "#,
        )
        .bind(score)
        .bind(username)
        .bind(user_uuid)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            error!("Failed to update user score: {}", e);
            ServiceError::Database(e.to_string())
        })?;

        let rank = self
            .get_user_rank(user_id)
            .await?
            .map(|(r, _)| r)
            .unwrap_or(0);

        debug!(
            "Updated user {} score to {}, new rank: {}",
            user_id, score, rank
        );
        Ok(rank)
    }

    pub async fn get_global_stats(&self) -> Result<GlobalStats> {
        let stats = sqlx::query_as::<_, GlobalStats>(
            r#"
            SELECT
                COALESCE(SUM(total_clicks), 0)::BIGINT as total_clicks,
                COUNT(*)::BIGINT as total_users,
                (SELECT COUNT(*)::BIGINT FROM sessions WHERE is_active = true) as active_sessions
            FROM users
            "#,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            error!("Failed to get global stats: {}", e);
            ServiceError::Database(e.to_string())
        })?;

        Ok(stats)
    }

    pub async fn get_leaderboard_cached(
        &self,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<LeaderboardEntry>> {
        let entries = sqlx::query_as::<_, LeaderboardEntry>(
            r#"
            SELECT
                rank::BIGINT as rank,
                user_id,
                username,
                total_clicks
            FROM leaderboard_top_1000
            WHERE rank > $1
            ORDER BY rank
            LIMIT $2
            "#,
        )
        .bind(offset as i64)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            error!("Failed to fetch cached leaderboard: {}", e);
            ServiceError::Database(e.to_string())
        })?;

        debug!("Fetched {} cached leaderboard entries", entries.len());
        Ok(entries)
    }

    pub async fn get_user_rank_cached(&self, user_id: &str) -> Result<Option<(i32, i64)>> {
        let cached_result = sqlx::query_as::<_, (i64, i64)>(
            r#"
            SELECT rank::BIGINT, total_clicks
            FROM leaderboard_top_1000
            WHERE user_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            error!("Failed to get cached user rank: {}", e);
            ServiceError::Database(e.to_string())
        })?;

        if let Some((rank, clicks)) = cached_result {
            return Ok(Some((rank as i32, clicks)));
        }

        debug!("User {} not in cached leaderboard, using real-time query", user_id);
        self.get_user_rank(user_id).await
    }

    pub async fn refresh_leaderboard_cache(&self) -> Result<()> {
        let start = std::time::Instant::now();

        sqlx::query("SELECT refresh_leaderboard()")
            .execute(&self.pool)
            .await
            .map_err(|e| {
                error!("Failed to refresh leaderboard cache: {}", e);
                ServiceError::Database(e.to_string())
            })?;

        let duration = start.elapsed();
        debug!("Refreshed leaderboard cache in {:?}", duration);

        Ok(())
    }
}
