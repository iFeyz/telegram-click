mod grpc_client;
mod rate_limiter;
mod state;
mod telegram;
mod websocket;

use axum::{routing::get, Router};
use grpc_client::{GameServiceClient, LeaderboardServiceClient, GrpcClientPool};
use state::State;
use std::env;
use std::sync::Arc;
use std::time::Duration;
use teloxide::dispatching::dialogue::InMemStorage;
use teloxide::prelude::*;
use tonic::transport::Channel;
use tower_http::services::ServeDir;
use tracing_subscriber;
use websocket::{AppState, LeaderboardBroadcaster};
use shared::config::BatchConfig;

type MyDialogue = Dialogue<State, InMemStorage<State>>;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();

    let jaeger_endpoint = std::env::var("JAEGER_ENDPOINT").ok();
    let metrics_port: u16 = std::env::var("METRICS_PORT")
        .unwrap_or_else(|_| "9091".to_string())
        .parse()
        .expect("METRICS_PORT must be a valid port number");

    shared::init_tracing("bot-service", jaeger_endpoint)?;

    shared::init_metrics(metrics_port)?;

    tracing::info!("Starting Bot Service...");

    let bot_token =
        env::var("TELOXIDE_TOKEN").expect("TELOXIDE_TOKEN environment variable not set");
    let game_service_url =
        env::var("GAME_SERVICE_URL").unwrap_or_else(|_| "http://localhost:50051".to_string());
    let leaderboard_service_url =
        env::var("LEADERBOARD_SERVICE_URL").unwrap_or_else(|_| "http://localhost:50052".to_string());
    let mini_app_url =
        env::var("MINI_APP_URL").unwrap_or_else(|_| "https://example.com/mini-app".to_string());
    let websocket_port: u16 = env::var("WEBSOCKET_PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse()
        .expect("WEBSOCKET_PORT must be a valid port number");


    let enable_telegram_polling = env::var("ENABLE_TELEGRAM_POLLING")
        .unwrap_or_else(|_| "true".to_string())
        .to_lowercase() == "true";

    let batch_config = BatchConfig::from_env()?;

    tracing::info!("Configuration:");
    tracing::info!("  Game Service URL: {}", game_service_url);
    tracing::info!("  Leaderboard Service URL: {}", leaderboard_service_url);
    tracing::info!("  Mini App URL: {}", mini_app_url);
    tracing::info!("  WebSocket Port: {}", websocket_port);
    tracing::info!("  Telegram Polling Enabled: {}", enable_telegram_polling);
    tracing::info!("  Leaderboard Broadcast Interval: {}ms", batch_config.leaderboard_broadcast_interval_ms);


    let grpc_pool_size: usize = env::var("GRPC_POOL_SIZE")
        .unwrap_or_else(|_| "20".to_string())
        .parse()
        .unwrap_or(20);

    tracing::info!("Creating gRPC connection pool (size: {})...", grpc_pool_size);

    tracing::info!("Connecting to Game Service pool...");
    let mut game_clients = Vec::new();
    for i in 0..grpc_pool_size {
        let channel = Channel::from_shared(game_service_url.clone())?
            .concurrency_limit(256)
            .initial_stream_window_size(1024 * 1024)
            .initial_connection_window_size(10 * 1024 * 1024)
            .tcp_nodelay(true)
            .http2_keep_alive_interval(Duration::from_secs(30))
            .timeout(Duration::from_millis(500))
            .connect()
            .await
            .map_err(|e| format!("Failed to connect to game-service client {}: {}", i, e))?;

        game_clients.push(GameServiceClient::new(channel));
        tracing::debug!("Connected game-service client {}/{}", i + 1, grpc_pool_size);
    }
    let game_client_pool = Arc::new(GrpcClientPool::new(game_clients));
    tracing::info!("Game Service pool ready ({} connections)", grpc_pool_size);

    tracing::info!("Connecting to Leaderboard Service pool...");
    let mut leaderboard_clients = Vec::new();
    for i in 0..grpc_pool_size {
        let channel = Channel::from_shared(leaderboard_service_url.clone())?
            .concurrency_limit(256)
            .tcp_nodelay(true)
            .connect()
            .await
            .map_err(|e| format!("Failed to connect to leaderboard-service client {}: {}", i, e))?;

        leaderboard_clients.push(LeaderboardServiceClient::new(channel));
        tracing::debug!("  ✓ Connected leaderboard-service client {}/{}", i + 1, grpc_pool_size);
    }
    let leaderboard_client_pool = Arc::new(GrpcClientPool::new(leaderboard_clients));
    tracing::info!("✅ Leaderboard Service pool ready ({} connections)", grpc_pool_size);

    let websocket_handle = tokio::spawn(run_websocket_server(
        game_client_pool,
        leaderboard_client_pool,
        websocket_port,
        batch_config.leaderboard_broadcast_interval_ms,
    ));

    if enable_telegram_polling {
        tracing::info!("This instance will handle Telegram polling");
        tracing::info!("Connecting single clients for Telegram bot...");
        let game_client_telegram = GameServiceClient::connect(game_service_url.clone()).await?;
        let leaderboard_client_telegram = LeaderboardServiceClient::connect(leaderboard_service_url.clone()).await?;
        tracing::info!("Telegram bot clients ready");

        let bot = Bot::new(bot_token);
        let bot_handle = tokio::spawn(run_telegram_bot(
            bot,
            game_client_telegram,
            leaderboard_client_telegram,
            mini_app_url.clone()
        ));

        tracing::info!("Bot Service is running");
        tracing::info!("  - Telegram bot: Active (POLLING)");
        tracing::info!("  - WebSocket server: ws://0.0.0.0:{}", websocket_port);

        tokio::select! {
            result = bot_handle => {
                if let Err(e) = result {
                    tracing::error!("Telegram bot task failed: {}", e);
                }
            }
            result = websocket_handle => {
                if let Err(e) = result {
                    tracing::error!("WebSocket server task failed: {}", e);
                }
            }
        }
    } else {
        tracing::info!("This instance will handle ONLY WebSocket connections (no Telegram polling)");
        tracing::info!("Bot Service is running");
        tracing::info!("  - Telegram bot: Disabled");
        tracing::info!("  - WebSocket server: ws://0.0.0.0:{}", websocket_port);

        if let Err(e) = websocket_handle.await {
            tracing::error!("WebSocket server task failed: {}", e);
        }
    }

    Ok(())
}

async fn run_telegram_bot(bot: Bot, game_client: GameServiceClient, leaderboard_client: LeaderboardServiceClient, mini_app_url: String) {
    tracing::info!("Starting Telegram bot...");

    let me = loop {
        match bot.get_me().await {
            Ok(me) => {
                tracing::info!("Bot username: @{}", me.username());
                break me;
            }
            Err(e) => {
                tracing::warn!("Failed to get bot info (will retry): {}", e);
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        }
    };

    let storage = InMemStorage::<State>::new();

    let game_client_idle = game_client.clone();
    let leaderboard_client_idle = leaderboard_client.clone();
    let mini_app_url_idle = mini_app_url.clone();
    let me_idle = me.clone();

    let game_client_name_change = game_client.clone();

    let game_client_cb = game_client;
    let leaderboard_client_cb = leaderboard_client;
    let mini_app_url_cb = mini_app_url;

    let handler = dptree::entry()
        .branch(
            Update::filter_message()
                .enter_dialogue::<Update, InMemStorage<State>, State>()
                .branch(dptree::case![State::Idle].endpoint(
                    move |bot: Bot, msg: Message, dialogue: MyDialogue| {
                        let game_client = game_client_idle.clone();
                        let leaderboard_client = leaderboard_client_idle.clone();
                        let mini_app_url = mini_app_url_idle.clone();
                        let me = me_idle.clone();
                        async move {
                            telegram::handlers::handle_idle_state(
                                bot,
                                msg,
                                dialogue,
                                me,
                                game_client,
                                leaderboard_client,
                                mini_app_url,
                            )
                            .await
                            .map_err(|e| {
                                tracing::error!("Idle state handler error: {}", e);
                                e
                            })
                        }
                    },
                ))
                .branch(
                    dptree::case![State::WaitingForNameChange { user_id }].endpoint(
                        move |bot: Bot, msg: Message, dialogue: MyDialogue, user_id: String| {
                            let game_client = game_client_name_change.clone();
                            async move {
                                telegram::handlers::handle_name_change_input(
                                    bot,
                                    msg,
                                    dialogue,
                                    user_id,
                                    game_client,
                                )
                                .await
                                .map_err(|e| {
                                    tracing::error!("Name change input handler error: {}", e);
                                    e
                                })
                            }
                        },
                    ),
                ),
        )
        .branch(
            Update::filter_callback_query()
                .enter_dialogue::<Update, InMemStorage<State>, State>()
                .endpoint(move |bot: Bot, q: CallbackQuery, dialogue: MyDialogue| {
                    let game_client = game_client_cb.clone();
                    let leaderboard_client = leaderboard_client_cb.clone();
                    let mini_app_url = mini_app_url_cb.clone();
                    async move {
                        telegram::handlers::handle_callback_query(
                            bot,
                            q,
                            dialogue,
                            game_client,
                            leaderboard_client,
                            mini_app_url,
                        )
                        .await
                        .map_err(|e| {
                            tracing::error!("Callback query handler error: {}", e);
                            e
                        })
                    }
                }),
        );

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![storage])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

async fn run_websocket_server(
    game_client_pool: Arc<GrpcClientPool<GameServiceClient>>,
    leaderboard_client_pool: Arc<GrpcClientPool<LeaderboardServiceClient>>,
    port: u16,
    broadcast_interval_ms: u64,
) {
    tracing::info!("Starting WebSocket server on port {}...", port);

    let (broadcast_tx, _) = tokio::sync::broadcast::channel(100);

    let leaderboard_broadcaster = Arc::new(LeaderboardBroadcaster::new(
        leaderboard_client_pool.clone(),
        broadcast_tx.clone(),
        broadcast_interval_ms,
    ));

    tracing::info!(
        interval_ms = broadcast_interval_ms,
        "Starting leaderboard broadcaster with connection pool"
    );
    leaderboard_broadcaster.clone().start_periodic_broadcaster();

    let app_state = AppState {
        game_client_pool,
        leaderboard_client_pool,
        broadcast_tx,
    };

    let app = Router::new()
        .route("/ws", get(websocket::websocket_handler))
        .route("/health", get(health_check))
        .with_state(app_state)
        .fallback_service(ServeDir::new("../mini-app/dist"));

    tracing::info!("Serving static files from ../mini-app/dist");

    let listener = match tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await {
        Ok(l) => {
            tracing::info!("WebSocket server listening on {}", l.local_addr().unwrap());
            l
        }
        Err(e) => {
            tracing::error!("Failed to bind WebSocket server: {}", e);
            return;
        }
    };

    if let Err(e) = axum::serve(listener, app).await {
        tracing::error!("WebSocket server error: {}", e);
    }
}

async fn health_check() -> &'static str {
    "OK"
}
