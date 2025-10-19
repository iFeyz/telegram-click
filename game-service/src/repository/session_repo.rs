use shared::{Result, ServiceError, Session, SessionId, SessionStats, UserId};
use sqlx::{PgPool, Row};
use chrono::{DateTime, Utc};


pub struct SessionRepository {
    pool: PgPool,
}

impl SessionRepository {
    /// Create a new session repository
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }


    pub async fn create_session(
        &self,
        user_id: &UserId,
        chat_id: i64,
        message_id: Option<i32>,
    ) -> Result<Session> {
        let row = sqlx::query(
            r#"
            INSERT INTO sessions (user_id, chat_id, message_id, started_at, last_heartbeat, is_active)
            VALUES ($1, $2, $3, NOW(), NOW(), TRUE)
            RETURNING id, user_id, chat_id, message_id, started_at, last_heartbeat, is_active
            "#,
        )
        .bind(user_id.0)
        .bind(chat_id)
        .bind(message_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(Session {
            id: SessionId(row.get("id")),
            user_id: UserId(row.get("user_id")),
            chat_id: row.get("chat_id"),
            message_id: row.get("message_id"),
            started_at: row.get("started_at"),
            last_heartbeat: row.get("last_heartbeat"),
            is_active: row.get("is_active"),
        })
    }



    pub async fn update_heartbeat(&self, session_id: &SessionId) -> Result<()> {
        let result = sqlx::query(
            r#"
            UPDATE sessions
            SET last_heartbeat = NOW()
            WHERE id = $1 AND is_active = TRUE
            "#,
        )
        .bind(session_id.0)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(ServiceError::SessionNotFound(session_id.to_string()));
        }

        Ok(())
    }


    pub async fn end_session(&self, session_id: &SessionId) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE sessions
            SET is_active = FALSE, ended_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(session_id.0)
        .execute(&self.pool)
        .await?;

        Ok(())
    }


    pub async fn increment_session_clicks(&self, session_id: &SessionId, count: i32) -> Result<()> {
        let result = sqlx::query(
            r#"
            UPDATE sessions
            SET total_clicks = total_clicks + $1,
                last_heartbeat = NOW()
            WHERE id = $2 AND is_active = TRUE
            "#,
        )
        .bind(count)
        .bind(session_id.0)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(ServiceError::SessionNotFound(session_id.to_string()));
        }

        Ok(())
    }

    pub async fn get_session_stats(&self, session_id: &SessionId) -> Result<SessionStats> {
        let row = sqlx::query(
            r#"
            SELECT
                id,
                user_id,
                chat_id,
                message_id,
                started_at,
                ended_at,
                last_heartbeat,
                total_clicks,
                is_active,
                EXTRACT(EPOCH FROM COALESCE(ended_at, NOW()) - started_at)::INT as duration_secs
            FROM sessions
            WHERE id = $1
            "#,
        )
        .bind(session_id.0)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| ServiceError::SessionNotFound(session_id.to_string()))?;

        Ok(SessionStats {
            session_id: SessionId(row.get("id")),
            user_id: UserId(row.get("user_id")),
            chat_id: row.get("chat_id"),
            message_id: row.get("message_id"),
            started_at: row.get("started_at"),
            ended_at: row.get("ended_at"),
            last_heartbeat: row.get("last_heartbeat"),
            total_clicks: row.get("total_clicks"),
            is_active: row.get("is_active"),
            duration_secs: row.get("duration_secs"),
        })
    }

    pub async fn get_active_session_for_user(
        &self,
        user_id: &UserId,
        timeout_secs: i64,
    ) -> Result<Option<Session>> {

        let row = sqlx::query!(
            r#"
            SELECT id, user_id, chat_id, message_id, started_at, last_heartbeat, is_active
            FROM sessions
            WHERE user_id = $1
            AND is_active = TRUE
            AND last_heartbeat > NOW() - $2 * INTERVAL '1 second'
            ORDER BY started_at DESC
            LIMIT 1
            "#,
            user_id.0,
            timeout_secs as f64
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| Session {
            id: SessionId(r.id),
            user_id: UserId(r.user_id),
            chat_id: r.chat_id,
            message_id: r.message_id,
            started_at: r.started_at,
            last_heartbeat: r.last_heartbeat,
            is_active: r.is_active,
        }))
    }

    pub async fn get_by_id(&self, session_id: &SessionId) -> Result<Session> {
        let row = sqlx::query(
            r#"
            SELECT id, user_id, chat_id, message_id, started_at, last_heartbeat, is_active
            FROM sessions
            WHERE id = $1
            "#,
        )
        .bind(session_id.0)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| ServiceError::SessionNotFound(session_id.to_string()))?;

        Ok(Session {
            id: SessionId(row.get("id")),
            user_id: UserId(row.get("user_id")),
            chat_id: row.get("chat_id"),
            message_id: row.get("message_id"),
            started_at: row.get("started_at"),
            last_heartbeat: row.get("last_heartbeat"),
            is_active: row.get("is_active"),
        })
    }

    pub async fn count_active_sessions(&self, timeout_secs: i64) -> Result<i64> {
        let row = sqlx::query(
            r#"
            SELECT COUNT(*) as count
            FROM sessions
            WHERE is_active = TRUE
            AND last_heartbeat > NOW() - $1 * INTERVAL '1 second'
            "#,
        )
        .bind(timeout_secs)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.get("count"))
    }

    pub async fn get_active_sessions(
        &self,
        limit: i64,
        offset: i64,
        timeout_secs: i64,
    ) -> Result<Vec<Session>> {
        let rows = sqlx::query(
            r#"
            SELECT id, user_id, chat_id, message_id, started_at, last_heartbeat, is_active
            FROM sessions
            WHERE is_active = TRUE
            AND last_heartbeat > NOW() - $1 * INTERVAL '1 second'
            ORDER BY last_heartbeat DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(timeout_secs)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let sessions = rows
            .into_iter()
            .map(|row| Session {
                id: SessionId(row.get("id")),
                user_id: UserId(row.get("user_id")),
                chat_id: row.get("chat_id"),
                message_id: row.get("message_id"),
                started_at: row.get("started_at"),
                last_heartbeat: row.get("last_heartbeat"),
                is_active: row.get("is_active"),
            })
            .collect();

        Ok(sessions)
    }

    pub async fn cleanup_expired_sessions(&self, timeout_secs: i64) -> Result<u64> {
        let result = sqlx::query(
            r#"
            UPDATE sessions
            SET is_active = FALSE
            WHERE is_active = TRUE
            AND last_heartbeat < NOW() - $1 * INTERVAL '1 second'
            "#,
        )
        .bind(timeout_secs)
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
    async fn test_session_lifecycle() {
        // Integration test placeholder
        // 1. Create session
        // 2. Update heartbeat
        // 3. End session
        // 4. Verify state
    }
}
