use leaderboard_service::grpc_server::leaderboard_server::game::leaderboard_service_server::LeaderboardServiceServer;
use leaderboard_service::{LeaderboardRepository, LeaderboardServerImpl};
use shared::errors::Result;
use sqlx::postgres::PgPoolOptions;
use std::env;
use tonic::transport::Server;
use tracing::{error, info};
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();

    let jaeger_endpoint = std::env::var("JAEGER_ENDPOINT").ok();
    let metrics_port: u16 = std::env::var("METRICS_PORT")
        .unwrap_or_else(|_| "9093".to_string())
        .parse()
        .expect("METRICS_PORT must be a valid port number");

    shared::init_tracing("leaderboard-service", jaeger_endpoint)
        .expect("Failed to initialize tracing");

    shared::init_metrics(metrics_port)
        .expect("Failed to initialize metrics");

    info!("Starting Leaderboard Service");

    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:password@localhost:5432/clickgame".to_string());
    let grpc_port = env::var("GRPC_PORT").unwrap_or_else(|_| "50052".to_string());

    info!("Configuration:");
    info!("  Database URL: {}", database_url.replace(|c: char| c.is_numeric(), "*"));
    info!("  gRPC Port: {}", grpc_port);

    info!("Connecting to PostgreSQL...");
    let db_pool = PgPoolOptions::new()
        .max_connections(150)
        .connect(&database_url)
        .await
        .map_err(|e| {
            error!("Failed to connect to PostgreSQL: {}", e);
            shared::errors::ServiceError::Database(e.to_string())
        })?;

    info!("Connected to PostgreSQL successfully");

    sqlx::query("SELECT 1")
        .execute(&db_pool)
        .await
        .map_err(|e| {
            error!("Database health check failed: {}", e);
            shared::errors::ServiceError::Database(e.to_string())
        })?;

    info!("Database health check passed");

    let repository = LeaderboardRepository::new(db_pool);

    let enable_refresh = env::var("ENABLE_CACHE_REFRESH")
        .unwrap_or_else(|_| "true".to_string())
        .to_lowercase() == "true";

    if enable_refresh {
        let refresh_interval_ms = env::var("LEADERBOARD_REFRESH_INTERVAL_MS")
            .unwrap_or_else(|_| "500".to_string())
            .parse::<u64>()
            .unwrap_or(500);

        info!("Starting leaderboard cache refresh task (interval: {}ms)", refresh_interval_ms);

        let repository_clone = repository.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_millis(refresh_interval_ms));
            loop {
                interval.tick().await;

                if let Err(e) = repository_clone.refresh_leaderboard_cache().await {
                    error!("Failed to refresh leaderboard cache: {}", e);
                } else {
                    info!("Leaderboard cache refreshed successfully");
                }
            }
        });
    } else {
        info!("Cache refresh task DISABLED (ENABLE_CACHE_REFRESH=false)");
    }

    let grpc_server = LeaderboardServerImpl::new(repository);
    let grpc_service = LeaderboardServiceServer::new(grpc_server);

    let addr = format!("0.0.0.0:{}", grpc_port).parse().map_err(|e| {
        error!("Failed to parse gRPC address: {}", e);
        shared::errors::ServiceError::Internal(format!("Invalid address: {}", e))
    })?;

    info!("Starting gRPC server on {}", addr);

    Server::builder()
        .add_service(grpc_service)
        .serve_with_shutdown(addr, async {
            tokio::signal::ctrl_c()
                .await
                .expect("Failed to listen for ctrl-c");
            info!("Shutdown signal received");
        })
        .await
        .map_err(|e| {
            error!("gRPC server error: {}", e);
            shared::errors::ServiceError::Internal(e.to_string())
        })?;

    info!("Leaderboard Service stopped");
    Ok(())
}
