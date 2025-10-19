use crate::grpc_client::{GameServiceClient, LeaderboardServiceClient, GrpcClientPool, get_shard_for_user};
use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::Response,
};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;

#[derive(Clone)]
pub struct AppState {
    pub game_client_pool: Arc<GrpcClientPool<GameServiceClient>>,
    pub leaderboard_client_pool: Arc<GrpcClientPool<LeaderboardServiceClient>>,
    pub broadcast_tx: broadcast::Sender<BroadcastMessage>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum BroadcastMessage {
    LeaderboardUpdate(ServerMessage),
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum ClientMessage {
    #[serde(rename = "init")]
    Init {
        user_id: String,
        telegram_id: i64,
        username: String,
    },
    #[serde(rename = "click")]
    Click {
        user_id: String,
        telegram_id: i64,
        session_id: String,
        click_count: Option<u32>,
    },
    #[serde(rename = "refresh")]
    Refresh {
        user_id: String,
        telegram_id: i64,
    },
}

#[derive(Debug, Serialize, Clone)]
pub struct LeaderboardEntry {
    pub rank: i32,
    pub username: String,
    #[serde(rename = "totalClicks")]
    pub total_clicks: i64,
}

#[derive(Debug, Serialize, Clone)]
#[serde(tag = "type")]
pub enum ServerMessage {
    #[serde(rename = "score_update")]
    ScoreUpdate {
        score: i64,
        rank: i32,
        user_id: Option<String>,
        username: Option<String>,
    },
    #[serde(rename = "session_info")]
    SessionInfo {
        session_id: String,
        is_reconnection: bool,
        started_at: i64,
    },
    #[serde(rename = "leaderboard_update")]
    LeaderboardUpdate {
        entries: Vec<LeaderboardEntry>,
    },
    #[serde(rename = "error")]
    Error { message: String },
    #[serde(rename = "rate_limited")]
    RateLimited { message: String },
}

pub async fn websocket_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let (sender, mut receiver) = socket.split();
    let sender = Arc::new(tokio::sync::Mutex::new(sender));
    let mut broadcast_rx = state.broadcast_tx.subscribe();

    tracing::info!("New WebSocket connection established");

    let sender_clone = Arc::clone(&sender);
    let mut broadcast_task = tokio::spawn(async move {
        while let Ok(broadcast_msg) = broadcast_rx.recv().await {
            match broadcast_msg {
                BroadcastMessage::LeaderboardUpdate(msg) => {
                    if let Ok(json) = serde_json::to_string(&msg) {
                        let mut sender_lock = sender_clone.lock().await;
                        if sender_lock.send(Message::Text(json.into())).await.is_err() {
                            break;
                        }
                    }
                }
            }
        }
    });

    let sender_clone = Arc::clone(&sender);
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if let Message::Text(text) = msg {
                tracing::debug!("Received WebSocket message: {}", text);

                match serde_json::from_str::<ClientMessage>(&text) {
                    Ok(client_msg) => {
                        let responses = handle_client_message(client_msg, &state).await;

                        for response in responses {
                            if let Ok(response_json) = serde_json::to_string(&response) {
                                let mut sender_lock = sender_clone.lock().await;
                                if sender_lock.send(Message::Text(response_json.into())).await.is_err() {
                                    tracing::error!("Failed to send response to client");
                                    break;
                                }
                            }
                        }


                    }
                    Err(e) => {
                        tracing::error!("Failed to parse client message: {}", e);
                        let error_response = ServerMessage::Error {
                            message: "Invalid message format".to_string(),
                        };
                        if let Ok(error_json) = serde_json::to_string(&error_response) {
                            let mut sender_lock = sender_clone.lock().await;
                            let _ = sender_lock.send(Message::Text(error_json.into())).await;
                        }
                    }
                }
            } else if let Message::Close(_) = msg {
                tracing::info!("WebSocket connection closed by client");
                break;
            }
        }
    });

    tokio::select! {
        _ = &mut broadcast_task => {
            recv_task.abort();
        }
        _ = &mut recv_task => {
            broadcast_task.abort();
        }
    }

    tracing::info!("WebSocket connection terminated");
}

#[tracing::instrument(skip(state), fields(msg_type = ?msg))]
async fn handle_client_message(msg: ClientMessage, state: &AppState) -> Vec<ServerMessage> {
    let overall_start = std::time::Instant::now();

    match msg {
        ClientMessage::Init {
            user_id: _,
            telegram_id,
            username,
        } => {
            let init_start = std::time::Instant::now();
            tracing::info!(
                telegram_id = telegram_id,
                username = %username,
                "WebSocket init request"
            );

            shared::record_counter("websocket.init.requests", 1);

            let user_fetch_start = std::time::Instant::now();
            let client_mutex = state.game_client_pool.get_client();
            let pool_select_time = user_fetch_start.elapsed();

            tracing::debug!(
                duration_ms = pool_select_time.as_millis(),
                "Got gRPC client from pool"
            );
            shared::record_timing("grpc.pool.get_client", pool_select_time.as_secs_f64());

            let mut client = client_mutex.lock().await;
            let lock_time = user_fetch_start.elapsed() - pool_select_time;

            tracing::debug!(
                duration_ms = lock_time.as_millis(),
                "Acquired gRPC client lock"
            );
            shared::record_timing("grpc.client.lock_wait", lock_time.as_secs_f64());

            let grpc_call_start = std::time::Instant::now();
            let user_response = client.get_user(telegram_id).await;
            let grpc_duration = grpc_call_start.elapsed();

            shared::record_timing("grpc.get_user", grpc_duration.as_secs_f64());

            match user_response {
                Ok(user_response) if user_response.exists => {
                    let user_fetch_time = user_fetch_start.elapsed();
                    tracing::info!(
                        "⏱️ User found in {:?} - user_id: {}, username: {}, total_clicks: {}",
                        user_fetch_time,
                        user_response.user_id,
                        user_response.username,
                        user_response.total_clicks
                    );

                    let session_start = std::time::Instant::now();
                    let client_mutex = state.game_client_pool.get_client();
                    let mut client = client_mutex.lock().await;
                    let session_lock_time = session_start.elapsed();
                    tracing::debug!("⏱️ Got session client lock in {:?}", session_lock_time);

                    let session_response = client.get_or_create_session(
                        user_response.user_id.clone(),
                        0,
                        None,
                    ).await;

                    match session_response {
                        Ok(session_response) if session_response.success => {
                            let session_time = session_start.elapsed();
                            tracing::info!(
                                "⏱️ Session {} in {:?} - session_id: {}, session_clicks: {}",
                                if session_response.is_reconnection { "resumed" } else { "created" },
                                session_time,
                                session_response.session_id,
                                session_response.total_clicks
                            );

                            let rank_fetch_start = std::time::Instant::now();
                            let leaderboard_client_mutex = state.leaderboard_client_pool.get_client();
                            let mut leaderboard_client = leaderboard_client_mutex.lock().await;

                            let rank = match leaderboard_client.get_user_rank(user_response.user_id.clone()).await {
                                Ok(rank_response) if rank_response.found => {
                                    let rank_fetch_time = rank_fetch_start.elapsed();
                                    tracing::debug!("⏱️ Got user rank in {:?}", rank_fetch_time);
                                    rank_response.rank
                                },
                                _ => {
                                    tracing::warn!("Failed to get rank for user {}", user_response.user_id);
                                    0
                                }
                            };

                            let total_time = init_start.elapsed();
                            tracing::info!("⏱️ TOTAL WebSocket init time: {:?}", total_time);

                            vec![
                                ServerMessage::SessionInfo {
                                    session_id: session_response.session_id,
                                    is_reconnection: session_response.is_reconnection,
                                    started_at: session_response.started_at,
                                },
                                ServerMessage::ScoreUpdate {
                                    score: user_response.total_clicks,
                                    rank,
                                    user_id: Some(user_response.user_id),
                                    username: Some(user_response.username),
                                },
                            ]
                        }
                        Ok(_) => {
                            tracing::error!("Failed to create/resume session");
                            vec![ServerMessage::Error {
                                message: "Failed to initialize session".to_string(),
                            }]
                        }
                        Err(e) => {
                            tracing::error!("Failed to get or create session: {}", e);
                            vec![ServerMessage::Error {
                                message: "Failed to initialize session".to_string(),
                            }]
                        }
                    }
                },
                Ok(_) => {
                    tracing::warn!("User not found for telegram_id: {}", telegram_id);
                    vec![ServerMessage::Error {
                        message: "Account not found. Please use /start command in the bot first.".to_string(),
                    }]
                },
                Err(e) => {
                    tracing::error!("Failed to get user: {}", e);
                    vec![ServerMessage::Error {
                        message: "Failed to fetch user data".to_string(),
                    }]
                }
            }
        }

        ClientMessage::Click {
            user_id,
            telegram_id,
            session_id,
            click_count,
        } => {
            let click_start = std::time::Instant::now();
            let batch_size = click_count.unwrap_or(1);

            tracing::debug!(
                user_id = %user_id,
                session_id = %session_id,
                batch_size = batch_size,
                "Processing click batch"
            );

            shared::record_counter("click.requests", 1);
            shared::record_counter("click.total_clicks", batch_size as u64);

            let pool_size = state.game_client_pool.size();
            let shard_index = get_shard_for_user(&user_id, pool_size);
            let client_mutex = state.game_client_pool.get_client_by_shard(shard_index);
            let pool_time = click_start.elapsed();

            tracing::debug!(
                shard = shard_index,
                pool_size = pool_size,
                duration_ms = pool_time.as_millis(),
                "Routed click to shard"
            );
            shared::record_timing("click.routing", pool_time.as_secs_f64());

            let mut client = client_mutex.lock().await;
            let lock_time = click_start.elapsed() - pool_time;

            tracing::debug!(
                duration_ms = lock_time.as_millis(),
                "Acquired gRPC client lock for click"
            );
            shared::record_timing("click.lock_wait", lock_time.as_secs_f64());

            let user_id_for_rank = user_id.clone();

            let grpc_call_start = std::time::Instant::now();
            let result = client.process_click(user_id, telegram_id, session_id, batch_size).await;
            let grpc_duration = grpc_call_start.elapsed();

            shared::record_timing("grpc.process_click", grpc_duration.as_secs_f64());

            let total_time = click_start.elapsed();
            shared::record_timing("click.total_latency", total_time.as_secs_f64());

            tracing::info!(
                total_ms = total_time.as_millis(),
                grpc_ms = grpc_duration.as_millis(),
                lock_ms = lock_time.as_millis(),
                routing_ms = pool_time.as_millis(),
                "Click processing complete"
            );

            match result {
                Ok(response) => {
                    if response.rate_limited {
                        shared::record_counter("click.rate_limited", 1);
                        vec![ServerMessage::RateLimited {
                            message: response.message,
                        }]
                    } else if response.success {
                        shared::record_counter("click.success", 1);

                        let rank_fetch_start = std::time::Instant::now();
                        let leaderboard_client_mutex = state.leaderboard_client_pool.get_client();
                        let mut leaderboard_client = leaderboard_client_mutex.lock().await;

                        let rank = match leaderboard_client.get_user_rank(user_id_for_rank.clone()).await {
                            Ok(rank_response) if rank_response.found => {
                                let rank_fetch_time = rank_fetch_start.elapsed();
                                tracing::debug!("⏱️ Got user rank in {:?}", rank_fetch_time);
                                rank_response.rank
                            },
                            _ => {
                                tracing::warn!("Failed to get rank for user {}, using 0", user_id_for_rank);
                                0
                            }
                        };

                        vec![ServerMessage::ScoreUpdate {
                            score: response.new_total,
                            rank,
                            user_id: None, // user_id already known by client
                            username: None, // username already known by client
                        }]
                    } else {
                        shared::record_counter("click.failed", 1);
                        vec![ServerMessage::Error {
                            message: response.message,
                        }]
                    }
                }
                Err(e) => {
                    shared::record_counter("click.errors", 1);
                    tracing::error!(
                        error = %e,
                        "Failed to process click"
                    );
                    vec![ServerMessage::Error {
                        message: "Failed to process click".to_string(),
                    }]
                }
            }
        }

        ClientMessage::Refresh {
            user_id,
            telegram_id,
        } => {
            let refresh_start = std::time::Instant::now();

            tracing::debug!(
                user_id = %user_id,
                telegram_id = telegram_id,
                "Processing refresh request"
            );

            shared::record_counter("refresh.requests", 1);

            let game_client_mutex = state.game_client_pool.get_client();
            let mut game_client = game_client_mutex.lock().await;

            let user_response = game_client.get_user(telegram_id).await;

            match user_response {
                Ok(user_response) if user_response.exists => {
                    let score = user_response.total_clicks;

                    let leaderboard_client_mutex = state.leaderboard_client_pool.get_client();
                    let mut leaderboard_client = leaderboard_client_mutex.lock().await;

                    let rank = match leaderboard_client.get_user_rank(user_id).await {
                        Ok(rank_response) if rank_response.found => rank_response.rank,
                        _ => {
                            tracing::warn!("Failed to get rank for user {}, using 0", user_response.user_id);
                            0
                        }
                    };

                    let total_time = refresh_start.elapsed();
                    shared::record_timing("refresh.total_latency", total_time.as_secs_f64());
                    shared::record_counter("refresh.success", 1);

                    tracing::info!(
                        total_ms = total_time.as_millis(),
                        score = score,
                        rank = rank,
                        "Refresh completed successfully"
                    );

                    vec![ServerMessage::ScoreUpdate {
                        score,
                        rank,
                        user_id: None,
                        username: None,
                    }]
                }
                Ok(_) => {
                    shared::record_counter("refresh.user_not_found", 1);
                    tracing::warn!("User not found for telegram_id: {}", telegram_id);
                    vec![ServerMessage::Error {
                        message: "User not found".to_string(),
                    }]
                }
                Err(e) => {
                    shared::record_counter("refresh.errors", 1);
                    tracing::error!(
                        error = %e,
                        "Failed to refresh user data"
                    );
                    vec![ServerMessage::Error {
                        message: "Failed to refresh data".to_string(),
                    }]
                }
            }
        }
    }
}
