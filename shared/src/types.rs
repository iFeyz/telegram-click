use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::errors::{Result, ServiceError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserId(pub Uuid);

impl UserId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_string(s: &str) -> Result<Self> {
        Uuid::parse_str(s)
            .map(UserId)
            .map_err(|e| ServiceError::Validation(format!("Invalid user ID: {}", e)))
    }

    pub fn to_string(&self) -> String {
        self.0.to_string()
    }
}

impl Default for UserId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for UserId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub Uuid);

impl SessionId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_string(s: &str) -> Result<Self> {
        Uuid::parse_str(s)
            .map(SessionId)
            .map_err(|e| ServiceError::Validation(format!("Invalid session ID: {}", e)))
    }

    pub fn to_string(&self) -> String {
        self.0.to_string()
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Username(String);

impl Username {
    const MIN_LENGTH: usize = 3;
    const MAX_LENGTH: usize = 20;

    pub fn new(username: impl Into<String>) -> Result<Self> {
        let username = username.into();
        Self::validate(&username)?;
        Ok(Self(username))
    }

    fn validate(username: &str) -> Result<()> {
        if username.len() < Self::MIN_LENGTH {
            return Err(ServiceError::InvalidUsername(format!(
                "Username must be at least {} characters",
                Self::MIN_LENGTH
            )));
        }

        if username.len() > Self::MAX_LENGTH {
            return Err(ServiceError::InvalidUsername(format!(
                "Username must be at most {} characters",
                Self::MAX_LENGTH
            )));
        }

        if !username
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        {
            return Err(ServiceError::InvalidUsername(
                "Username can only contain letters, numbers, underscores, and hyphens".to_string(),
            ));
        }

        Ok(())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Username {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: UserId,
    pub telegram_id: i64,
    pub username: Username,
    pub total_clicks: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: SessionId,
    pub user_id: UserId,
    pub chat_id: i64,
    pub message_id: Option<i32>,
    pub started_at: DateTime<Utc>,
    pub last_heartbeat: DateTime<Utc>,
    pub is_active: bool,
}

impl Session {
    pub fn new(user_id: UserId, chat_id: i64) -> Self {
        let now = Utc::now();
        Self {
            id: SessionId::new(),
            user_id,
            chat_id,
            message_id: None,
            started_at: now,
            last_heartbeat: now,
            is_active: true,
        }
    }

    pub fn is_expired(&self, timeout_secs: i64) -> bool {
        let elapsed = Utc::now().signed_duration_since(self.last_heartbeat);
        elapsed.num_seconds() > timeout_secs
    }

    pub fn update_heartbeat(&mut self) {
        self.last_heartbeat = Utc::now();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStats {
    pub session_id: SessionId,
    pub user_id: UserId,
    pub chat_id: i64,
    pub message_id: Option<i32>,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub last_heartbeat: DateTime<Utc>,
    pub total_clicks: i32,
    pub is_active: bool,
    pub duration_secs: i32,
}

impl SessionStats {
    pub fn clicks_per_minute(&self) -> f32 {
        if self.duration_secs == 0 {
            return 0.0;
        }
        (self.total_clicks as f32 / self.duration_secs as f32) * 60.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClickEvent {
    pub user_id: UserId,
    pub session_id: SessionId,
    pub timestamp: DateTime<Utc>,
    pub count: i32,
}

impl ClickEvent {
    pub fn new(user_id: UserId, session_id: SessionId) -> Self {
        Self {
            user_id,
            session_id,
            timestamp: Utc::now(),
            count: 1,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderboardEntry {
    pub rank: i32,
    pub username: String,
    pub total_clicks: i64,
    pub user_id: UserId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalStats {
    pub total_clicks: i64,
    pub total_users: i64,
    pub active_sessions: i64,
}
