pub mod config;
pub mod errors;
pub mod telemetry;
pub mod types;

pub use config::{DatabaseConfig, RedisConfig, ServiceConfig};
pub use errors::{Result, ServiceError};
pub use telemetry::{init_metrics, init_tracing, record_counter, record_gauge, record_timing, shutdown};
pub use types::{
    ClickEvent, GlobalStats, LeaderboardEntry, Session, SessionId, SessionStats, User, UserId,
    Username,
};

pub mod proto {
    tonic::include_proto!("game");
}
