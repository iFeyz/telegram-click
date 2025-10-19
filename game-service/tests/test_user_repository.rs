
mod common;

use common::create_test_user_data;
use game_service::repository::UserRepository;
use shared::Username;
use sqlx::PgPool;
use anyhow::Result;

#[sqlx::test(migrations = "../migrations")]
async fn test_create_user_success(pool: PgPool) -> Result<()> {
    let repo = UserRepository::new(pool);
    let (telegram_id, username) = create_test_user_data("create");

    let user = repo.create_user(telegram_id, &username).await?;

    assert_eq!(user.telegram_id, telegram_id);
    assert_eq!(user.username.as_str(), username);
    assert_eq!(user.total_clicks, 0);

    Ok(())
}

#[sqlx::test(migrations = "../migrations")]
async fn test_create_user_duplicate_telegram_id(pool: PgPool) -> Result<()> {
    let repo = UserRepository::new(pool);
    let (telegram_id, username) = create_test_user_data("duplicate");

    repo.create_user(telegram_id, &username).await?;

    let result = repo.create_user(telegram_id, "different_name").await;

    assert!(result.is_err(), "Should fail on duplicate telegram_id");
    assert!(result.unwrap_err().to_string().contains("already exists"));

    Ok(())
}

#[sqlx::test(migrations = "../migrations")]
async fn test_get_by_telegram_id_success(pool: PgPool) -> Result<()> {
    let repo = UserRepository::new(pool);
    let (telegram_id, username) = create_test_user_data("get");

    let created = repo.create_user(telegram_id, &username).await?;
    let fetched = repo.get_by_telegram_id(telegram_id).await?;

    assert_eq!(fetched.id, created.id);
    assert_eq!(fetched.telegram_id, telegram_id);
    assert_eq!(fetched.username.as_str(), username);

    Ok(())
}

#[sqlx::test(migrations = "../migrations")]
async fn test_get_by_telegram_id_not_found(pool: PgPool) -> Result<()> {
    let repo = UserRepository::new(pool);

    let result = repo.get_by_telegram_id(999999999).await;

    assert!(result.is_err(), "Should fail for non-existent user");
    assert!(result.unwrap_err().to_string().contains("not found"));

    Ok(())
}

#[sqlx::test(migrations = "../migrations")]
async fn test_update_username(pool: PgPool) -> Result<()> {
    let repo = UserRepository::new(pool);
    let (telegram_id, username) = create_test_user_data("update");

    let user = repo.create_user(telegram_id, &username).await?;
    let new_username = Username::new("updated_name").unwrap();

    repo.update_username(&user.id, &new_username).await?;

    let updated = repo.get_by_id(&user.id).await?;
    assert_eq!(updated.username.as_str(), "updated_name");

    Ok(())
}

#[sqlx::test(migrations = "../migrations")]
async fn test_increment_clicks_atomic(pool: PgPool) -> Result<()> {
    let repo = UserRepository::new(pool);
    let (telegram_id, username) = create_test_user_data("clicks");

    let user = repo.create_user(telegram_id, &username).await?;

    let mut results = Vec::new();
    for _ in 0..5 {
        let new_total = repo.increment_clicks(&user.id).await?;
        results.push(new_total);
    }

    assert_eq!(results, vec![1, 2, 3, 4, 5]);

    let final_user = repo.get_by_id(&user.id).await?;
    assert_eq!(final_user.total_clicks, 5);

    Ok(())
}

#[sqlx::test(migrations = "../migrations")]
async fn test_count_total_users(pool: PgPool) -> Result<()> {
    let repo = UserRepository::new(pool);

    for i in 0..3 {
        let suffix = format!("count_{}", i);
        let (telegram_id, username) = create_test_user_data(&suffix);
        repo.create_user(telegram_id, &username).await?;
    }

    let count = repo.count_total_users().await?;
    assert_eq!(count, 3);

    Ok(())
}
