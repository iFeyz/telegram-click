use crate::grpc_client::GameServiceClient;
use crate::state::State;
use crate::telegram::{format_welcome_message, make_game_keyboard, make_username_keyboard};
use shared::errors::{Result, ServiceError};
use teloxide::{
    dispatching::dialogue::InMemStorage,
    prelude::*,
    types::{CallbackQuery, Me, Message},
    utils::command::BotCommands,
};

type MyDialogue = Dialogue<State, InMemStorage<State>>;

fn map_teloxide_err<E: std::fmt::Display>(e: E) -> ServiceError {
    ServiceError::Telegram(e.to_string())
}

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Available commands:")]
pub enum Command {
    #[command(description = "Start the bot and register")]
    Start,
    #[command(description = "Change your username")]
    Changename,
    #[command(description = "Refresh your score and rank")]
    Refresh,
}

pub async fn handle_idle_state(
    bot: Bot,
    msg: Message,
    dialogue: MyDialogue,
    me: Me,
    game_client: GameServiceClient,
    leaderboard_client: crate::grpc_client::LeaderboardServiceClient,
    mini_app_url: String,
) -> Result<()> {
    if let Some(text) = msg.text() {
        match BotCommands::parse(text, me.username()) {
            Ok(Command::Start) => {
                handle_start(bot, msg, game_client, leaderboard_client, mini_app_url).await?;
            }
            Ok(Command::Changename) => {
                handle_changename_command(bot, msg, dialogue, game_client).await?;
            }
            Ok(Command::Refresh) => {
                handle_refresh(bot, msg, game_client, leaderboard_client).await?;
            }
            Err(_) => {
            }
        }
    }

    Ok(())
}

pub async fn handle_callback_query(
    bot: Bot,
    q: CallbackQuery,
    dialogue: MyDialogue,
    game_client: GameServiceClient,
    leaderboard_client: crate::grpc_client::LeaderboardServiceClient,
    mini_app_url: String,
) -> Result<()> {
    if let Some(data) = &q.data {
        match data.as_str() {
            "change_name" => {
                if let Some(msg) = &q.message {
                    let chat = msg.chat();

                    let telegram_id = q.from.id.0 as i64;
                    let mut client = game_client.clone();

                    match client.get_user(telegram_id).await {
                        Ok(user_response) if user_response.exists => {
                            dialogue
                                .update(State::WaitingForNameChange {
                                    user_id: user_response.user_id,
                                })
                                .await
                                .map_err(|e| {
                                    ServiceError::Internal(format!(
                                        "Failed to update dialogue: {}",
                                        e
                                    ))
                                })?;

                            bot.send_message(
                                chat.id,
                                "Please send me your new username:\n\n\
                                📝 Requirements:\n\
                                • 3-20 characters\n\
                                • Letters, numbers, underscore, hyphen only\n\n\
                                Send /cancel to abort.",
                            )
                            .await
                            .map_err(map_teloxide_err)?;
                        }
                        _ => {
                            bot.send_message(chat.id, "❌ Please /start first!")
                                .await
                                .map_err(map_teloxide_err)?;
                        }
                    }
                }
            }
            "refresh" => {
                tracing::info!("Refresh button clicked");
            }
            "username_random" => {
                let random_username = generate_random_username();
                if let Some(msg) = &q.message {
                    let chat = msg.chat();
                    create_user_and_show_welcome(
                        bot.clone(),
                        chat.id,
                        q.from.id.0 as i64,
                        random_username,
                        game_client,
                        leaderboard_client,
                        mini_app_url,
                    )
                    .await?;
                }
            }
            "username_custom" => {
                if let Some(msg) = &q.message {
                    let chat = msg.chat();
                    bot.send_message(
                        chat.id,
                        "Please send me your desired username (3-20 characters, alphanumeric only):",
                    )
                    .await
                    .map_err(map_teloxide_err)?;
                }
            }
            _ => {}
        }

        bot.answer_callback_query(q.id)
            .await
            .map_err(map_teloxide_err)?;
    }

    Ok(())
}

async fn handle_start(
    bot: Bot,
    msg: Message,
    mut game_client: GameServiceClient,
    leaderboard_client: crate::grpc_client::LeaderboardServiceClient,
    mini_app_url: String,
) -> Result<()> {
    let start_time = std::time::Instant::now();
    let telegram_id = msg.from.as_ref().map(|u| u.id.0 as i64).unwrap_or(0);

    tracing::info!("⏱️ /start BEGIN for telegram_id: {}", telegram_id);

    let user_fetch_start = std::time::Instant::now();
    let user_response = game_client.get_user(telegram_id).await?;
    tracing::info!("⏱️ get_user took: {:?}", user_fetch_start.elapsed());

    if user_response.exists {
        let welcome_start = std::time::Instant::now();
        send_welcome_message(bot, msg, user_response, leaderboard_client, mini_app_url).await?;
        tracing::info!("⏱️ send_welcome_message took: {:?}", welcome_start.elapsed());
    } else {
        bot.send_message(
            msg.chat.id,
            "👋 Welcome to Bitcoin Clicker!\n\nChoose how to set your username:",
        )
        .reply_markup(make_username_keyboard())
        .await
        .map_err(map_teloxide_err)?;
    }

    tracing::info!("⏱️ /start TOTAL time: {:?}", start_time.elapsed());
    Ok(())
}

async fn handle_changename_command(
    bot: Bot,
    msg: Message,
    dialogue: MyDialogue,
    mut game_client: GameServiceClient,
) -> Result<()> {
    let telegram_id = msg.from.as_ref().map(|u| u.id.0 as i64).unwrap_or(0);

    let user_response = game_client.get_user(telegram_id).await?;

    if !user_response.exists {
        bot.send_message(msg.chat.id, "❌ Please /start first to register!")
            .await
            .map_err(map_teloxide_err)?;
        return Ok(());
    }

    dialogue
        .update(State::WaitingForNameChange {
            user_id: user_response.user_id,
        })
        .await
        .map_err(|e| ServiceError::Internal(format!("Failed to update dialogue: {}", e)))?;

    bot.send_message(
        msg.chat.id,
        "Please send me your new username:\n\n\
        📝 Requirements:\n\
        • 3-20 characters\n\
        • Letters, numbers, underscore, hyphen only\n\
        • No spaces\n\n\
        Send /cancel to abort.",
    )
    .await
    .map_err(map_teloxide_err)?;

    Ok(())
}

async fn handle_refresh(
    bot: Bot,
    msg: Message,
    mut game_client: GameServiceClient,
    mut leaderboard_client: crate::grpc_client::LeaderboardServiceClient,
) -> Result<()> {
    let refresh_start = std::time::Instant::now();

    let telegram_id = msg.from.as_ref().map(|u| u.id.0 as i64).unwrap_or(0);

    tracing::info!("⏱️ /refresh BEGIN for telegram_id: {}", telegram_id);

    let user_fetch_start = std::time::Instant::now();
    let user_response = game_client.get_user(telegram_id).await?;
    tracing::info!("⏱️ get_user took: {:?}", user_fetch_start.elapsed());

    if !user_response.exists {
        bot.send_message(msg.chat.id, "❌ Please /start first to register!")
            .await
            .map_err(map_teloxide_err)?;
        return Ok(());
    }

    let rank_fetch_start = std::time::Instant::now();
    let rank_response = leaderboard_client.get_user_rank(user_response.user_id.clone()).await?;
    let rank = if rank_response.found {
        rank_response.rank
    } else {
        0
    };
    tracing::info!("⏱️ get_user_rank took: {:?}", rank_fetch_start.elapsed());

    let message = format!(
        "🔄 *Stats Refreshed!*\n\n\
        👤 *{}*\n\
        🏆 Rank: *#{}*\n\
        💎 Total Clicks: *{}*\n\n\
        _Updated at {}_",
        user_response.username,
        rank,
        user_response.total_clicks,
        chrono::Utc::now().format("%H:%M:%S UTC")
    );

    bot.send_message(msg.chat.id, message)
        .parse_mode(teloxide::types::ParseMode::MarkdownV2)
        .await
        .map_err(map_teloxide_err)?;

    tracing::info!("⏱️ /refresh TOTAL time: {:?}", refresh_start.elapsed());
    Ok(())
}

pub async fn handle_name_change_input(
    bot: Bot,
    msg: Message,
    dialogue: MyDialogue,
    user_id: String,
    mut game_client: GameServiceClient,
) -> Result<()> {
    if msg.text() == Some("/cancel") {
        dialogue.update(State::Idle).await.ok();
        bot.send_message(msg.chat.id, "❌ Username change cancelled.")
            .await
            .map_err(map_teloxide_err)?;
        return Ok(());
    }

    let new_username = match msg.text() {
        Some(text) => text.trim().to_string(),
        None => {
            bot.send_message(msg.chat.id, "❌ Please send text, not other content.")
                .await
                .map_err(map_teloxide_err)?;
            return Ok(());
        }
    };

    if !is_valid_username(&new_username) {
        bot.send_message(
            msg.chat.id,
            "❌ Invalid username!\n\n\
            Requirements:\n\
            • 3-20 characters\n\
            • Letters, numbers, underscore (_), hyphen (-) only\n\n\
            Please try again or send /cancel:",
        )
        .await
        .map_err(map_teloxide_err)?;
        return Ok(());
    }

    let response = game_client
        .update_username(user_id.clone(), new_username.clone())
        .await?;

    if !response.success {
        bot.send_message(
            msg.chat.id,
            format!("❌ Failed to change username: {}", response.message),
        )
        .await
        .map_err(map_teloxide_err)?;

        return Ok(());
    }

    dialogue.update(State::Idle).await.ok();

    bot.send_message(
        msg.chat.id,
        format!("✅ Username changed to: {}", new_username),
    )
    .await
    .map_err(map_teloxide_err)?;

    Ok(())
}

fn is_valid_username(username: &str) -> bool {
    let len = username.len();
    if len < 3 || len > 20 {
        return false;
    }

    username
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
}

async fn send_welcome_message(
    bot: Bot,
    msg: Message,
    user_data: crate::grpc_client::game_client::GetUserResponse,
    leaderboard_client: crate::grpc_client::LeaderboardServiceClient,
    mini_app_url: String,
) -> Result<()> {
    let mut leaderboard_client_mut = leaderboard_client.clone();
    let (leaderboard, user_rank, global_clicks) =
        match fetch_leaderboard_data(&mut leaderboard_client_mut, &user_data.user_id).await {
            Ok(data) => data,
            Err(e) => {
                tracing::warn!("Failed to fetch leaderboard data: {}, using placeholder", e);
                (vec![], 0, user_data.total_clicks)
            }
        };

    let text = format_welcome_message(
        &user_data.username,
        user_data.total_clicks,
        global_clicks,
        user_rank,
        &leaderboard,
    );

    let keyboard = make_game_keyboard(&mini_app_url);

    bot.send_message(msg.chat.id, text)
        .reply_markup(keyboard)
        .await
        .map_err(map_teloxide_err)?;

    Ok(())
}

async fn fetch_leaderboard_data(
    leaderboard_client: &mut crate::grpc_client::LeaderboardServiceClient,
    user_id: &str,
) -> Result<(Vec<(i32, String, i64)>, i32, i64)> {
    let fetch_start = std::time::Instant::now();

    let mut leaderboard_client_clone = leaderboard_client.clone();
    let user_id_clone = user_id.to_string();

    let concurrent_start = std::time::Instant::now();
    let (leaderboard_result, rank_result) = tokio::join!(
        leaderboard_client.get_leaderboard(Some(20), Some(0)),
        leaderboard_client_clone.get_user_rank(user_id_clone)
    );
    tracing::info!("⏱️ Concurrent calls (leaderboard + rank) took: {:?}", concurrent_start.elapsed());

    let leaderboard_response = leaderboard_result?;
    let leaderboard: Vec<(i32, String, i64)> = leaderboard_response
        .entries
        .iter()
        .map(|entry| (entry.rank, entry.username.clone(), entry.total_clicks))
        .collect();

    let rank_response = rank_result?;
    let user_rank = if rank_response.found {
        rank_response.rank
    } else {
        0
    };

    let stats_start = std::time::Instant::now();
    let stats_response = leaderboard_client.get_global_stats().await?;
    tracing::info!("⏱️ get_global_stats took: {:?}", stats_start.elapsed());
    let global_clicks = stats_response.total_clicks;

    tracing::info!("⏱️ fetch_leaderboard_data TOTAL: {:?}", fetch_start.elapsed());
    Ok((leaderboard, user_rank, global_clicks))
}

async fn create_user_and_show_welcome(
    bot: Bot,
    chat_id: ChatId,
    telegram_id: i64,
    username: String,
    mut game_client: GameServiceClient,
    leaderboard_client: crate::grpc_client::LeaderboardServiceClient,
    mini_app_url: String,
) -> Result<()> {
    let create_response = game_client.create_user(telegram_id, username).await?;

    if !create_response.success {
        bot.send_message(chat_id, format!("Error: {}", create_response.message))
            .await
            .map_err(map_teloxide_err)?;
        return Ok(());
    }

    let user_response = game_client.get_user(telegram_id).await?;

    let mut leaderboard_client_mut = leaderboard_client.clone();
    let (leaderboard, user_rank, global_clicks) =
        match fetch_leaderboard_data(&mut leaderboard_client_mut, &user_response.user_id).await {
            Ok(data) => data,
            Err(e) => {
                tracing::warn!("Failed to fetch leaderboard data: {}, using placeholder", e);
                (vec![], 0, user_response.total_clicks)
            }
        };

    let text = format_welcome_message(
        &user_response.username,
        user_response.total_clicks,
        global_clicks,
        user_rank,
        &leaderboard,
    );

    let keyboard = make_game_keyboard(&mini_app_url);

    bot.send_message(chat_id, text)
        .reply_markup(keyboard)
        .await
        .map_err(map_teloxide_err)?;

    Ok(())
}

fn generate_random_username() -> String {
    use chrono::Utc;
    let timestamp = Utc::now().timestamp() % 10000;
    format!("Player{}", timestamp)
}
