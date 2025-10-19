use shared::{ClickEvent, Result, ServiceError, SessionId, UserId};
use sqlx::{PgPool, Row};


#[derive(Clone)]
pub struct ClickRepository {
    pool: PgPool,
}

impl ClickRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn record_click(&self, user_id: &UserId, session_id: &SessionId) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO clicks (user_id, session_id, timestamp, click_count)
            VALUES ($1, $2, NOW(), 1)
            "#,
        )
        .bind(user_id.0)
        .bind(session_id.0)
        .execute(&self.pool)
        .await?;

        Ok(())
    }


    pub async fn record_clicks_batch(&self, events: &[ClickEvent]) -> Result<u64> {
        if events.is_empty() {
            return Ok(0);
        }

        // Build multi-row insert query
        let mut query_builder = sqlx::QueryBuilder::new(
            "INSERT INTO clicks (user_id, session_id, timestamp, click_count) "
        );

        query_builder.push_values(events, |mut b, event| {
            b.push_bind(event.user_id.0)
                .push_bind(event.session_id.0)
                .push_bind(event.timestamp)
                .push_bind(event.count);
        });

        let result = query_builder.build().execute(&self.pool).await?;

        Ok(result.rows_affected())
    }


    pub async fn get_recent_click_count(&self, user_id: &UserId, minutes: i32) -> Result<i64> {
        let row = sqlx::query(
            r#"
            SELECT COALESCE(SUM(click_count), 0) as total
            FROM clicks
            WHERE user_id = $1
            AND timestamp > NOW() - $2 * INTERVAL '1 minute'
            "#,
        )
        .bind(user_id.0)
        .bind(minutes)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.get("total"))
    }

    pub async fn get_global_click_count(&self) -> Result<i64> {
        let row = sqlx::query(
            r#"
            SELECT COALESCE(SUM(click_count), 0) as total
            FROM clicks
            "#,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(row.get("total"))
    }


    pub async fn cleanup_old_clicks(&self, days: i32) -> Result<u64> {
        let result = sqlx::query(
            r#"
            DELETE FROM clicks
            WHERE timestamp < NOW() - $1 * INTERVAL '1 day'
            "#,
        )
        .bind(days)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires database
    async fn test_record_click() {
        // Integration test placeholder
    }
}
