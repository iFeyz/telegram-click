
mod common;

use common::create_test_user_data;
use game_service::repository::{SessionRepository, UserRepository};
use sqlx::PgPool;
use anyhow::Result;
use tokio::time::{sleep, Duration};

#[sqlx::test(migrations = "../migrations")]
async fn test_create_session_success(pool: PgPool) -> Result<()> {
    let user_repo = UserRepository::new(pool.clone());
    let session_repo = SessionRepository::new(pool);

    let (telegram_id, username) = create_test_user_data("session_create");
    let user = user_repo.create_user(telegram_id, &username).await?;

    let chat_id = 123456;
    let message_id = Some(789);
    let session = session_repo
        .create_session(&user.id, chat_id, message_id)
        .await?;

    assert_eq!(session.user_id, user.id);
    assert_eq!(session.chat_id, chat_id);
    assert_eq!(session.message_id, message_id);
    assert!(session.is_active);

    Ok(())
}

#[sqlx::test(migrations = "../migrations")]
async fn test_get_session_by_id(pool: PgPool) -> Result<()> {
    let user_repo = UserRepository::new(pool.clone());
    let session_repo = SessionRepository::new(pool);

    let (telegram_id, username) = create_test_user_data("session_get");
    let user = user_repo.create_user(telegram_id, &username).await?;

    let created_session = session_repo
        .create_session(&user.id, 123456, None)
        .await?;

    let fetched_session = session_repo.get_by_id(&created_session.id).await?;

    assert_eq!(fetched_session.id, created_session.id);
    assert_eq!(fetched_session.user_id, user.id);
    assert_eq!(fetched_session.chat_id, 123456);
    assert!(fetched_session.is_active);

    Ok(())
}

#[sqlx::test(migrations = "../migrations")]
async fn test_get_session_not_found(pool: PgPool) -> Result<()> {
    use shared::SessionId;
    use uuid::Uuid;

    let session_repo = SessionRepository::new(pool);

    let non_existent_id = SessionId(Uuid::new_v4());
    let result = session_repo.get_by_id(&non_existent_id).await;

    assert!(result.is_err(), "Should fail for non-existent session");
    assert!(result.unwrap_err().to_string().contains("not found"));

    Ok(())
}

#[sqlx::test(migrations = "../migrations")]
async fn test_update_heartbeat(pool: PgPool) -> Result<()> {
    let user_repo = UserRepository::new(pool.clone());
    let session_repo = SessionRepository::new(pool);

    let (telegram_id, username) = create_test_user_data("session_heartbeat");
    let user = user_repo.create_user(telegram_id, &username).await?;

    let session = session_repo
        .create_session(&user.id, 123456, None)
        .await?;

    let original_heartbeat = session.last_heartbeat;

    sleep(Duration::from_millis(100)).await;

    session_repo.update_heartbeat(&session.id).await?;

    let updated_session = session_repo.get_by_id(&session.id).await?;
    assert!(updated_session.last_heartbeat > original_heartbeat);

    Ok(())
}

#[sqlx::test(migrations = "../migrations")]
async fn test_update_heartbeat_inactive_session(pool: PgPool) -> Result<()> {
    let user_repo = UserRepository::new(pool.clone());
    let session_repo = SessionRepository::new(pool);

    let (telegram_id, username) = create_test_user_data("session_inactive");
    let user = user_repo.create_user(telegram_id, &username).await?;

    let session = session_repo
        .create_session(&user.id, 123456, None)
        .await?;

    session_repo.end_session(&session.id).await?;

    let result = session_repo.update_heartbeat(&session.id).await;

    assert!(result.is_err(), "Should fail for inactive session");
    assert!(result.unwrap_err().to_string().contains("not found"));

    Ok(())
}

#[sqlx::test(migrations = "../migrations")]
async fn test_end_session(pool: PgPool) -> Result<()> {
    let user_repo = UserRepository::new(pool.clone());
    let session_repo = SessionRepository::new(pool);

    let (telegram_id, username) = create_test_user_data("session_end");
    let user = user_repo.create_user(telegram_id, &username).await?;

    let session = session_repo
        .create_session(&user.id, 123456, None)
        .await?;

    assert!(session.is_active);

    session_repo.end_session(&session.id).await?;

    let ended_session = session_repo.get_by_id(&session.id).await?;
    assert!(!ended_session.is_active);

    Ok(())
}

#[sqlx::test(migrations = "../migrations")]
async fn test_count_active_sessions(pool: PgPool) -> Result<()> {
    let user_repo = UserRepository::new(pool.clone());
    let session_repo = SessionRepository::new(pool);

    for i in 0..3 {
        let suffix = format!("active_{}", i);
        let (telegram_id, username) = create_test_user_data(&suffix);
        let user = user_repo.create_user(telegram_id, &username).await?;
        session_repo.create_session(&user.id, 123456, None).await?;
    }

    let count = session_repo.count_active_sessions(60).await?;
    assert_eq!(count, 3);

    Ok(())
}

#[sqlx::test(migrations = "../migrations")]
async fn test_get_active_sessions(pool: PgPool) -> Result<()> {
    let user_repo = UserRepository::new(pool.clone());
    let session_repo = SessionRepository::new(pool);

    for i in 0..5 {
        let suffix = format!("get_active_{}", i);
        let (telegram_id, username) = create_test_user_data(&suffix);
        let user = user_repo.create_user(telegram_id, &username).await?;
        session_repo.create_session(&user.id, 123456, None).await?;
    }

    let sessions = session_repo.get_active_sessions(3, 0, 60).await?;
    assert_eq!(sessions.len(), 3);

    let sessions_offset = session_repo.get_active_sessions(3, 3, 60).await?;
    assert_eq!(sessions_offset.len(), 2);

    Ok(())
}

#[sqlx::test(migrations = "../migrations")]
async fn test_cleanup_expired_sessions(pool: PgPool) -> Result<()> {
    let user_repo = UserRepository::new(pool.clone());
    let session_repo = SessionRepository::new(pool);

    let (telegram_id, username) = create_test_user_data("cleanup");
    let user = user_repo.create_user(telegram_id, &username).await?;
    let session = session_repo.create_session(&user.id, 123456, None).await?;

    sleep(Duration::from_millis(100)).await;

    let cleaned = session_repo.cleanup_expired_sessions(0).await?;
    assert_eq!(cleaned, 1);

    let expired_session = session_repo.get_by_id(&session.id).await?;
    assert!(!expired_session.is_active);

    Ok(())
}

#[sqlx::test(migrations = "../migrations")]
async fn test_cleanup_does_not_affect_recent_sessions(pool: PgPool) -> Result<()> {
    let user_repo = UserRepository::new(pool.clone());
    let session_repo = SessionRepository::new(pool);

$    let (telegram_id, username) = create_test_user_data("cleanup_recent");
    let user = user_repo.create_user(telegram_id, &username).await?;
    let session = session_repo.create_session(&user.id, 123456, None).await?;

    let cleaned = session_repo.cleanup_expired_sessions(3600).await?;
    assert_eq!(cleaned, 0);

    let active_session = session_repo.get_by_id(&session.id).await?;
    assert!(active_session.is_active);

    Ok(())
}
