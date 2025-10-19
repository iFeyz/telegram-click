
mod common;

use common::create_test_user_data;
use game_service::repository::{ClickRepository, SessionRepository, UserRepository};
use shared::ClickEvent;
use sqlx::PgPool;
use anyhow::Result;
use chrono::Utc;
use tokio::time::{sleep, Duration};

#[sqlx::test(migrations = "../migrations")]
async fn test_record_click_success(pool: PgPool) -> Result<()> {
    let user_repo = UserRepository::new(pool.clone());
    let session_repo = SessionRepository::new(pool.clone());
    let click_repo = ClickRepository::new(pool);

    let (telegram_id, username) = create_test_user_data("click_record");
    let user = user_repo.create_user(telegram_id, &username).await?;
    let session = session_repo.create_session(&user.id, 123456, None).await?;

    click_repo.record_click(&user.id, &session.id).await?;

    let count = click_repo.get_recent_click_count(&user.id, 10).await?;
    assert_eq!(count, 1);

    Ok(())
}

#[sqlx::test(migrations = "../migrations")]
async fn test_record_multiple_clicks(pool: PgPool) -> Result<()> {
    let user_repo = UserRepository::new(pool.clone());
    let session_repo = SessionRepository::new(pool.clone());
    let click_repo = ClickRepository::new(pool);

    let (telegram_id, username) = create_test_user_data("click_multiple");
    let user = user_repo.create_user(telegram_id, &username).await?;
    let session = session_repo.create_session(&user.id, 123456, None).await?;

    for _ in 0..5 {
        click_repo.record_click(&user.id, &session.id).await?;
    }

    let count = click_repo.get_recent_click_count(&user.id, 10).await?;
    assert_eq!(count, 5);

    Ok(())
}

#[sqlx::test(migrations = "../migrations")]
async fn test_record_clicks_batch(pool: PgPool) -> Result<()> {
    let user_repo = UserRepository::new(pool.clone());
    let session_repo = SessionRepository::new(pool.clone());
    let click_repo = ClickRepository::new(pool);

    let (telegram_id, username) = create_test_user_data("click_batch");
    let user = user_repo.create_user(telegram_id, &username).await?;
    let session = session_repo.create_session(&user.id, 123456, None).await?;

    // Create batch of click events
    let events: Vec<ClickEvent> = (0..10)
        .map(|_| ClickEvent {
            user_id: user.id.clone(),
            session_id: session.id.clone(),
            timestamp: Utc::now(),
            count: 1,
        })
        .collect();

    let rows_inserted = click_repo.record_clicks_batch(&events).await?;
    assert_eq!(rows_inserted, 10);

    let count = click_repo.get_recent_click_count(&user.id, 10).await?;
    assert_eq!(count, 10);

    Ok(())
}

#[sqlx::test(migrations = "../migrations")]
async fn test_record_clicks_batch_empty(pool: PgPool) -> Result<()> {
    let click_repo = ClickRepository::new(pool);

    let events: Vec<ClickEvent> = vec![];
    let rows_inserted = click_repo.record_clicks_batch(&events).await?;
    assert_eq!(rows_inserted, 0);

    Ok(())
}

#[sqlx::test(migrations = "../migrations")]
async fn test_get_recent_click_count_time_window(pool: PgPool) -> Result<()> {
    let user_repo = UserRepository::new(pool.clone());
    let session_repo = SessionRepository::new(pool.clone());
    let click_repo = ClickRepository::new(pool);

    let (telegram_id, username) = create_test_user_data("click_time_window");
    let user = user_repo.create_user(telegram_id, &username).await?;
    let session = session_repo.create_session(&user.id, 123456, None).await?;

    for _ in 0..3 {
        click_repo.record_click(&user.id, &session.id).await?;
    }

    let count_large = click_repo.get_recent_click_count(&user.id, 60).await?;
    assert_eq!(count_large, 3);

    let count_zero = click_repo.get_recent_click_count(&user.id, 0).await?;
    assert_eq!(count_zero, 0);

    Ok(())
}

#[sqlx::test(migrations = "../migrations")]
async fn test_get_recent_click_count_no_clicks(pool: PgPool) -> Result<()> {
    let user_repo = UserRepository::new(pool.clone());
    let click_repo = ClickRepository::new(pool);

    let (telegram_id, username) = create_test_user_data("click_no_clicks");
    let user = user_repo.create_user(telegram_id, &username).await?;

    let count = click_repo.get_recent_click_count(&user.id, 10).await?;
    assert_eq!(count, 0);

    Ok(())
}

#[sqlx::test(migrations = "../migrations")]
async fn test_get_global_click_count(pool: PgPool) -> Result<()> {
    let user_repo = UserRepository::new(pool.clone());
    let session_repo = SessionRepository::new(pool.clone());
    let click_repo = ClickRepository::new(pool);

    for i in 0..3 {
        let suffix = format!("global_{}", i);
        let (telegram_id, username) = create_test_user_data(&suffix);
        let user = user_repo.create_user(telegram_id, &username).await?;
        let session = session_repo.create_session(&user.id, 123456, None).await?;

        for _ in 0..2 {
            click_repo.record_click(&user.id, &session.id).await?;
        }
    }

    let global_count = click_repo.get_global_click_count().await?;
    assert_eq!(global_count, 6);

    Ok(())
}

#[sqlx::test(migrations = "../migrations")]
async fn test_get_global_click_count_empty(pool: PgPool) -> Result<()> {
    let click_repo = ClickRepository::new(pool);

    let global_count = click_repo.get_global_click_count().await?;
    assert_eq!(global_count, 0);

    Ok(())
}

#[sqlx::test(migrations = "../migrations")]
async fn test_cleanup_old_clicks(pool: PgPool) -> Result<()> {
    let user_repo = UserRepository::new(pool.clone());
    let session_repo = SessionRepository::new(pool.clone());
    let click_repo = ClickRepository::new(pool);

    let (telegram_id, username) = create_test_user_data("click_cleanup");
    let user = user_repo.create_user(telegram_id, &username).await?;
    let session = session_repo.create_session(&user.id, 123456, None).await?;

    for _ in 0..5 {
        click_repo.record_click(&user.id, &session.id).await?;
    }

    sleep(Duration::from_millis(100)).await;

    let deleted = click_repo.cleanup_old_clicks(0).await?;
    assert_eq!(deleted, 5);

    let count = click_repo.get_global_click_count().await?;
    assert_eq!(count, 0);

    Ok(())
}

#[sqlx::test(migrations = "../migrations")]
async fn test_cleanup_does_not_affect_recent_clicks(pool: PgPool) -> Result<()> {
    let user_repo = UserRepository::new(pool.clone());
    let session_repo = SessionRepository::new(pool.clone());
    let click_repo = ClickRepository::new(pool);

    let (telegram_id, username) = create_test_user_data("click_cleanup_recent");
    let user = user_repo.create_user(telegram_id, &username).await?;
    let session = session_repo.create_session(&user.id, 123456, None).await?;

     Record some clicks
    for _ in 0..3 {
        click_repo.record_click(&user.id, &session.id).await?;
    }

    let deleted = click_repo.cleanup_old_clicks(365).await?;
    assert_eq!(deleted, 0);

    let count = click_repo.get_global_click_count().await?;
    assert_eq!(count, 3);

    Ok(())
}

#[sqlx::test(migrations = "../migrations")]
async fn test_clicks_per_user_isolation(pool: PgPool) -> Result<()> {
    let user_repo = UserRepository::new(pool.clone());
    let session_repo = SessionRepository::new(pool.clone());
    let click_repo = ClickRepository::new(pool);

    let (telegram_id1, username1) = create_test_user_data("click_user1");
    let user1 = user_repo.create_user(telegram_id1, &username1).await?;
    let session1 = session_repo.create_session(&user1.id, 123456, None).await?;

    let (telegram_id2, username2) = create_test_user_data("click_user2");
    let user2 = user_repo.create_user(telegram_id2, &username2).await?;
    let session2 = session_repo.create_session(&user2.id, 123456, None).await?;

    for _ in 0..3 {
        click_repo.record_click(&user1.id, &session1.id).await?;
    }

    for _ in 0..5 {
        click_repo.record_click(&user2.id, &session2.id).await?;
    }

    let count1 = click_repo.get_recent_click_count(&user1.id, 10).await?;
    let count2 = click_repo.get_recent_click_count(&user2.id, 10).await?;

    assert_eq!(count1, 3);
    assert_eq!(count2, 5);

    let global = click_repo.get_global_click_count().await?;
    assert_eq!(global, 8);

    Ok(())
}
