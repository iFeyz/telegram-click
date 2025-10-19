
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::env;


pub async fn create_test_pool() -> PgPool {
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| {
            "postgres://postgres:password@localhost/clickgame_test".to_string()
        });

    PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to test database. Make sure PostgreSQL is running.")
}

pub async fn run_migrations(pool: &PgPool) {
    sqlx::migrate!("../migrations")
        .run(pool)
        .await
        .expect("Failed to run migrations");
}


pub async fn cleanup_test_data(pool: &PgPool) {
    sqlx::query("TRUNCATE TABLE clicks, sessions, users CASCADE")
        .execute(pool)
        .await
        .expect("Failed to cleanup test data");
}


pub fn create_test_user_data(suffix: &str) -> (i64, String) {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    suffix.hash(&mut hasher);
    let telegram_id = 1000000 + (hasher.finish() % 1000000) as i64;

    let username = if suffix.len() <= 10 {
        format!("test_{}", suffix)
    } else {
        format!("test_{}", &suffix[..10])
    };

    (telegram_id, username)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_database_connection() {
        let pool = create_test_pool().await;

        let result: (i32,) = sqlx::query_as("SELECT 1")
            .fetch_one(&pool)
            .await
            .expect("Failed to execute query");

        assert_eq!(result.0, 1);
    }
}
