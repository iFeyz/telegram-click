

use std::net::SocketAddr;
use tonic::transport::Server;
use sqlx::postgres::PgPoolOptions;
use redis::Client as RedisClient;

use shared::proto::game_service_server::GameServiceServer;
use shared::config::BatchConfig;
use game_service::{
    domain::RateLimiter,
    repository::{UserRepository, ClickRepository, SessionRepository},
    service::{UserService, ClickService, SessionService, RedisClickAccumulator},
    grpc_server::GameServerImpl,
    stream::ClickEventPublisher,
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();



    let jaeger_endpoint = std::env::var("JAEGER_ENDPOINT").ok();
    let metrics_port: u16 = std::env::var("METRICS_PORT")
        .unwrap_or_else(|_| "9092".to_string())
        .parse()
        .expect("METRICS_PORT must be a valid port number");

    shared::init_tracing("game-service", jaeger_endpoint)?;

    shared::init_metrics(metrics_port)?;

    tracing::info!("Game Service starting...");

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:password@localhost/clickgame".to_string());

    let redis_url = std::env::var("REDIS_URL")
        .unwrap_or_else(|_| "redis://localhost:6379".to_string());

    let port: u16 = std::env::var("GAME_SERVICE_PORT")
        .unwrap_or_else(|_| "50051".to_string())
        .parse()
        .expect("Invalid GAME_SERVICE_PORT");

    let click_rate_limit: u32 = std::env::var("CLICK_RATE_LIMIT")
        .unwrap_or_else(|_| "10".to_string())
        .parse()
        .expect("Invalid CLICK_RATE_LIMIT");

    let session_timeout: i64 = std::env::var("SESSION_TIMEOUT_SECS")
        .unwrap_or_else(|_| "60".to_string())
        .parse()
        .expect("Invalid SESSION_TIMEOUT_SECS");

    let batch_config = BatchConfig::from_env()?;

    let instance_id = std::env::var("INSTANCE_ID")
        .unwrap_or_else(|_| "game-1".to_string());

    let shard_id: usize = instance_id
        .split('-')
        .nth(1)
        .and_then(|s| s.parse::<usize>().ok())
        .map(|n| n - 1)
        .unwrap_or(0);

    let num_shards: usize = std::env::var("NUM_SHARDS")
        .unwrap_or_else(|_| "3".to_string())
        .parse()
        .expect("Invalid NUM_SHARDS");

    tracing::info!(
        database_url = %database_url,
        redis_url = %redis_url,
        port = port,
        click_rate_limit = click_rate_limit,
        session_timeout = session_timeout,
        click_flush_interval_ms = batch_config.click_flush_interval_ms,
        instance_id = %instance_id,
        shard_id = shard_id,
        num_shards = num_shards,
        "Configuration loaded"
    );


    tracing::info!("Connecting to PostgreSQL via PgBouncer...");
    let db_pool = PgPoolOptions::new()
        .max_connections(100)
        .min_connections(10)    
        .acquire_timeout(std::time::Duration::from_secs(5))
        .idle_timeout(std::time::Duration::from_secs(300))
        .max_lifetime(std::time::Duration::from_secs(1800))
        .connect(&database_url)
        .await?;

    tracing::info!("Connected to PostgreSQL successfully");

    let run_migrations = std::env::var("RUN_MIGRATIONS")
        .unwrap_or_else(|_| "true".to_string())
        .parse::<bool>()
        .unwrap_or(true);

    if run_migrations {
        tracing::info!("Running database migrations...");
        sqlx::migrate!("../migrations")
            .run(&db_pool)
            .await?;
        tracing::info!("Migrations completed");
    } else {
        tracing::info!("Skipping migrations (RUN_MIGRATIONS=false)");
    }

    tracing::info!("Connecting to Redis...");
    let redis_client = RedisClient::open(redis_url)?;
    let redis_conn_rate_limiter = redis_client.get_multiplexed_tokio_connection().await?;
    let redis_conn_publisher = redis_client.get_multiplexed_tokio_connection().await?;
    let redis_conn_accumulator = redis_client.get_multiplexed_tokio_connection().await?;
    tracing::info!("Connected to Redis successfully (3 multiplexed connections)");

    let rate_limiter = Arc::new(tokio::sync::Mutex::new(
        RateLimiter::new(redis_conn_rate_limiter, click_rate_limit)
    ));

    let event_publisher = ClickEventPublisher::new(redis_conn_publisher);
    tracing::info!("Initialized Redis Streams publisher");

    let user_repo = UserRepository::new(db_pool.clone());
    let click_repo = ClickRepository::new(db_pool.clone());
    let session_repo = SessionRepository::new(db_pool.clone());

    let batch_accumulator = Arc::new(RedisClickAccumulator::new(
        redis_conn_accumulator,
        UserRepository::new(db_pool.clone()),
        Some(event_publisher),
        batch_config.click_flush_interval_ms,
        shard_id,
        num_shards,
    ));

    tracing::info!(
        interval_ms = batch_config.click_flush_interval_ms,
        "Starting Redis-based click batch flusher (distributed)"
    );
    batch_accumulator.clone().start_background_flusher();

    let user_service = UserService::new(user_repo);
    let click_service = ClickService::new(
        UserRepository::new(db_pool.clone()),
        SessionRepository::new(db_pool.clone()),
        rate_limiter,
        batch_accumulator,
    );
    let session_service = SessionService::new(session_repo, session_timeout);

    let game_server = GameServerImpl::new(user_service, click_service, session_service);

    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse()?;

    tracing::info!(
        addr = %addr,
        "Starting gRPC server"
    );

    let cleanup_pool = db_pool.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            match sqlx::query!(
                "UPDATE sessions
                 SET is_active = false, ended_at = NOW()
                 WHERE is_active = true
                 AND last_heartbeat < NOW() - INTERVAL '5 minutes'"
            )
            .execute(&cleanup_pool)
            .await
            {
                Ok(result) => {
                    let rows = result.rows_affected();
                    if rows > 0 {
                        tracing::info!(
                            cleaned_sessions = rows,
                            "Cleaned up stale sessions"
                        );
                    }
                }
                Err(e) => {
                    tracing::error!(
                        error = %e,
                        "Failed to cleanup stale sessions"
                    );
                }
            }
        }
    });

    tracing::info!("Started session cleanup background task");

    Server::builder()
        .add_service(GameServiceServer::new(game_server))
        .serve(addr)
        .await?;

    tracing::info!("Server shut down gracefully");

    Ok(())
}
