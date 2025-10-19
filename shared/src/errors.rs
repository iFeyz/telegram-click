use thiserror::Error;

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("User not found: {0}")]
    UserNotFound(String),

    #[error("User already exists: {0}")]
    UserAlreadyExists(String),

    #[error("Invalid username: {0}")]
    InvalidUsername(String),

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Session expired: {0}")]
    SessionExpired(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Redis error: {0}")]
    Redis(String),

    #[error("gRPC error: {0}")]
    Grpc(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Telegram API error: {0}")]
    Telegram(String),
}

impl From<sqlx::Error> for ServiceError {
    fn from(err: sqlx::Error) -> Self {
        ServiceError::Database(err.to_string())
    }
}

impl From<redis::RedisError> for ServiceError {
    fn from(err: redis::RedisError) -> Self {
        ServiceError::Redis(err.to_string())
    }
}

impl From<tonic::Status> for ServiceError {
    fn from(err: tonic::Status) -> Self {
        ServiceError::Grpc(err.message().to_string())
    }
}

impl From<ServiceError> for tonic::Status {
    fn from(err: ServiceError) -> Self {
        match err {
            ServiceError::UserNotFound(msg) => tonic::Status::not_found(msg),
            ServiceError::UserAlreadyExists(msg) => tonic::Status::already_exists(msg),
            ServiceError::InvalidUsername(msg) => tonic::Status::invalid_argument(msg),
            ServiceError::RateLimitExceeded => {
                tonic::Status::resource_exhausted("Rate limit exceeded")
            }
            ServiceError::SessionNotFound(msg) => tonic::Status::not_found(msg),
            ServiceError::SessionExpired(msg) => tonic::Status::deadline_exceeded(msg),
            ServiceError::Database(msg) => tonic::Status::internal(format!("Database error: {}", msg)),
            ServiceError::Redis(msg) => tonic::Status::internal(format!("Redis error: {}", msg)),
            ServiceError::Grpc(msg) => tonic::Status::internal(msg),
            ServiceError::Validation(msg) => tonic::Status::invalid_argument(msg),
            ServiceError::Internal(msg) => tonic::Status::internal(msg),
            ServiceError::Telegram(msg) => tonic::Status::internal(format!("Telegram error: {}", msg)),
        }
    }
}

pub type Result<T> = std::result::Result<T, ServiceError>;
