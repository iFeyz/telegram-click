use shared::{Result, ServiceError, User, UserId, Username};
use sqlx::{PgPool, Row};

#[derive(Clone)]
pub struct UserRepository {
    pool: PgPool,
}

impl UserRepository {

    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }


    pub async fn create_user(&self, telegram_id: i64, username: &str) -> Result<User> {
        let row = sqlx::query(
            r#"
            INSERT INTO users (telegram_id, username, total_clicks)
            VALUES ($1, $2, 0)
            RETURNING id, telegram_id, username, total_clicks, created_at, updated_at
            "#,
        )
        .bind(telegram_id)
        .bind(username)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("duplicate key") {
                ServiceError::UserAlreadyExists(telegram_id.to_string())
            } else {
                ServiceError::Database(e.to_string())
            }
        })?;

        Ok(User {
            id: UserId(row.get("id")),
            telegram_id: row.get("telegram_id"),
            username: Username::new(row.get::<String, _>("username"))?,
            total_clicks: row.get("total_clicks"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }


    
    pub async fn get_by_telegram_id(&self, telegram_id: i64) -> Result<User> {
        let row = sqlx::query(
            r#"
            SELECT id, telegram_id, username, total_clicks, created_at, updated_at
            FROM users
            WHERE telegram_id = $1
            "#,
        )
        .bind(telegram_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| ServiceError::UserNotFound(telegram_id.to_string()))?;

        Ok(User {
            id: UserId(row.get("id")),
            telegram_id: row.get("telegram_id"),
            username: Username::new(row.get::<String, _>("username"))?,
            total_clicks: row.get("total_clicks"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }


    pub async fn get_by_id(&self, user_id: &UserId) -> Result<User> {
        let row = sqlx::query(
            r#"
            SELECT id, telegram_id, username, total_clicks, created_at, updated_at
            FROM users
            WHERE id = $1
            "#,
        )
        .bind(user_id.0)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| ServiceError::UserNotFound(user_id.to_string()))?;

        Ok(User {
            id: UserId(row.get("id")),
            telegram_id: row.get("telegram_id"),
            username: Username::new(row.get::<String, _>("username"))?,
            total_clicks: row.get("total_clicks"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }


    pub async fn update_username(&self, user_id: &UserId, username: &Username) -> Result<()> {
        let result = sqlx::query(
            r#"
            UPDATE users
            SET username = $1, updated_at = NOW()
            WHERE id = $2
            "#,
        )
        .bind(username.as_str())
        .bind(user_id.0)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(ServiceError::UserNotFound(user_id.to_string()));
        }

        Ok(())
    }


    pub async fn increment_clicks(&self, user_id: &UserId) -> Result<i64> {
        let row = sqlx::query(
            r#"
            UPDATE users
            SET total_clicks = total_clicks + 1, updated_at = NOW()
            WHERE id = $1
            RETURNING total_clicks
            "#,
        )
        .bind(user_id.0)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.get("total_clicks"))
    }



    pub async fn bulk_increment_clicks(
        &self,
        batches: &std::collections::HashMap<String, crate::service::UserClickBatch>,
    ) -> Result<std::collections::HashMap<String, i64>> {
        use std::collections::HashMap;

        if batches.is_empty() {
            return Ok(HashMap::new());
        }

    
        let mut sorted_batches: Vec<_> = batches.iter().collect();
        sorted_batches.sort_by_key(|(user_id_str, _)| *user_id_str);

      
        let mut query = String::from(
            "UPDATE users AS u \
             SET total_clicks = total_clicks + v.increment::bigint, updated_at = NOW() \
             FROM (VALUES "
        );

        let mut bind_values: Vec<(uuid::Uuid, i64)> = Vec::new();
        let mut first = true;

        for (user_id_str, batch) in sorted_batches.iter() {
            let user_id = uuid::Uuid::parse_str(user_id_str).map_err(|e| {
                ServiceError::Internal(format!("Invalid user_id UUID: {}", e))
            })?;

            if !first {
                query.push_str(", ");
            }
            first = false;

            let param_idx = bind_values.len();
            query.push_str(&format!("(${}, ${})", param_idx * 2 + 1, param_idx * 2 + 2));

            bind_values.push((user_id, batch.accumulated_clicks as i64));
        }

        query.push_str(") AS v(user_id, increment) WHERE u.id = v.user_id RETURNING u.id, u.total_clicks");

    
        let mut query_builder = sqlx::query(&query);
        for (user_id, increment) in bind_values.iter() {
            query_builder = query_builder.bind(user_id).bind(increment);
        }

        let rows = query_builder.fetch_all(&self.pool).await.map_err(|e| {
            tracing::error!(error = %e, "Bulk click increment failed");
            ServiceError::Database(e.to_string())
        })?;

        let mut result_map = HashMap::new();
        for row in rows {
            let user_id: uuid::Uuid = row.get("id");
            let total_clicks: i64 = row.get("total_clicks");
            result_map.insert(user_id.to_string(), total_clicks);
        }

        tracing::debug!(
            users_updated = result_map.len(),
            batches_submitted = batches.len(),
            "Bulk click increment completed"
        );

        Ok(result_map)
    }


    pub async fn count_total_users(&self) -> Result<i64> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM users")
            .fetch_one(&self.pool)
            .await?;

        Ok(row.get("count"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;


    #[tokio::test]
    #[ignore]
    async fn test_create_user() {
    }
}
