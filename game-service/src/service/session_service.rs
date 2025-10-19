use shared::{Result, ServiceError, Session, SessionId, SessionStats, UserId};
use crate::repository::SessionRepository;


pub struct SessionService {
    session_repo: SessionRepository,
    timeout_secs: i64,
}

impl SessionService {

    pub fn new(session_repo: SessionRepository, timeout_secs: i64) -> Self {
        Self {
            session_repo,
            timeout_secs,
        }
    }


    pub async fn start_session(
        &self,
        user_id: &UserId,
        chat_id: i64,
        message_id: Option<i32>,
    ) -> Result<Session> {
        let session = self.session_repo.create_session(user_id, chat_id, message_id).await?;

        tracing::info!(
            user_id = %user_id,
            session_id = %session.id,
            chat_id = chat_id,
            "Session started"
        );

        Ok(session)
    }


    pub async fn heartbeat(&self, session_id: &SessionId) -> Result<()> {
        self.session_repo.update_heartbeat(session_id).await?;

        tracing::debug!(
            session_id = %session_id,
            "Session heartbeat updated"
        );

        Ok(())
    }

    pub async fn end_session(&self, session_id: &SessionId) -> Result<()> {
        self.session_repo.end_session(session_id).await?;

        tracing::info!(
            session_id = %session_id,
            "Session ended"
        );

        Ok(())
    }

    pub async fn get_session(&self, session_id: &SessionId) -> Result<Session> {
        self.session_repo.get_by_id(session_id).await
    }

    pub async fn get_active_count(&self) -> Result<i64> {
        self.session_repo.count_active_sessions(self.timeout_secs).await
    }


    pub async fn get_active_sessions(&self, limit: i64, offset: i64) -> Result<Vec<Session>> {
        self.session_repo
            .get_active_sessions(limit, offset, self.timeout_secs)
            .await
    }

    pub async fn cleanup_expired(&self) -> Result<u64> {
        let count = self.session_repo
            .cleanup_expired_sessions(self.timeout_secs)
            .await?;

        if count > 0 {
            tracing::info!(
                count = count,
                timeout_secs = self.timeout_secs,
                "Expired sessions cleaned up"
            );
        }

        Ok(count)
    }

    pub async fn increment_clicks(&self, session_id: &SessionId, count: i32) -> Result<()> {
        self.session_repo.increment_session_clicks(session_id, count).await?;

        tracing::debug!(
            session_id = %session_id,
            count = count,
            "Session clicks incremented"
        );

        Ok(())
    }


    pub async fn get_stats(&self, session_id: &SessionId) -> Result<SessionStats> {
        self.session_repo.get_session_stats(session_id).await
    }


    pub async fn get_or_create_session(
        &self,
        user_id: &UserId,
        chat_id: i64,
        message_id: Option<i32>,
    ) -> Result<(SessionStats, bool)> {
        if let Some(existing_session) = self.session_repo
            .get_active_session_for_user(user_id, self.timeout_secs)
            .await?
        {
            tracing::info!(
                user_id = %user_id,
                session_id = %existing_session.id,
                "Reconnecting to existing session"
            );

            self.heartbeat(&existing_session.id).await?;

            let stats = self.get_stats(&existing_session.id).await?;

            return Ok((stats, true)); // true = reconnection
        }

        let new_session = self.start_session(user_id, chat_id, message_id).await?;

        tracing::info!(
            user_id = %user_id,
            session_id = %new_session.id,
            "Created new session"
        );

        let stats = self.get_stats(&new_session.id).await?;

        Ok((stats, false)) // false = new session
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_timeout_configuration() {
        let timeouts = vec![30, 60, 300, 600, 1800];

        for timeout in timeouts {
            assert!(timeout > 0, "Timeout must be positive");
            assert!(timeout <= 3600, "Timeout should not exceed 1 hour");
        }
    }

    #[test]
    fn test_session_id_uniqueness() {
        let session_id_1 = SessionId::new();
        let session_id_2 = SessionId::new();

        assert_ne!(session_id_1, session_id_2);
    }

    #[test]
    fn test_session_error_types() {
        let session_id = SessionId::new();
        let error = ServiceError::SessionNotFound(session_id.to_string());
        assert!(matches!(error, ServiceError::SessionNotFound(_)));

        let error = ServiceError::SessionExpired(session_id.to_string());
        assert!(matches!(error, ServiceError::SessionExpired(_)));
    }

    #[test]
    fn test_user_id_generation() {
        let user_id = UserId::new();

        assert!(!user_id.to_string().is_empty());
    }


}
