use shared::errors::{Result, ServiceError};
use tonic::transport::Channel;
use std::time::Duration;

pub mod game {
    tonic::include_proto!("game");
}

use game::game_service_client::GameServiceClient as GrpcGameServiceClient;
pub use game::*;

#[derive(Clone)]
pub struct GameServiceClient {
    client: GrpcGameServiceClient<Channel>,
}

impl GameServiceClient {
    pub fn new(channel: Channel) -> Self {
        let client = GrpcGameServiceClient::new(channel);
        Self { client }
    }

    pub async fn connect(url: String) -> Result<Self> {
        let endpoint = Channel::from_shared(url.clone())
            .map_err(|e| ServiceError::Grpc(format!("Invalid URL {}: {}", url, e)))?
            .connect_timeout(Duration::from_millis(1000))
            .timeout(Duration::from_millis(500))
            .concurrency_limit(256)
            .initial_stream_window_size(1024 * 1024)
            .initial_connection_window_size(10 * 1024 * 1024)
            .tcp_nodelay(true)
            .tcp_keepalive(Some(Duration::from_secs(30)))
            .http2_keep_alive_interval(Duration::from_secs(30))
            .keep_alive_timeout(Duration::from_secs(10))
            .keep_alive_while_idle(true);

        let client = GrpcGameServiceClient::connect(endpoint)
            .await
            .map_err(|e| {
                ServiceError::Grpc(format!(
                    "Failed to connect to Game Service at {}: {}",
                    url, e
                ))
            })?;

        tracing::info!("Connected to Game Service at {}", url);

        Ok(Self { client })
    }

    pub async fn create_user(
        &mut self,
        telegram_id: i64,
        username: String,
    ) -> Result<CreateUserResponse> {
        let request = tonic::Request::new(CreateUserRequest {
            telegram_id,
            username,
        });

        let response = self.client.create_user(request).await?.into_inner();

        Ok(response)
    }

    pub async fn get_user(&mut self, telegram_id: i64) -> Result<GetUserResponse> {
        let request = tonic::Request::new(GetUserRequest { telegram_id });

        let response = self.client.get_user(request).await?.into_inner();

        Ok(response)
    }

    pub async fn update_username(
        &mut self,
        user_id: String,
        new_username: String,
    ) -> Result<UpdateUsernameResponse> {
        let request = tonic::Request::new(UpdateUsernameRequest {
            user_id,
            new_username,
        });

        let response = self.client.update_username(request).await?.into_inner();

        Ok(response)
    }

    pub async fn process_click(
        &mut self,
        user_id: String,
        telegram_id: i64,
        session_id: String,
        click_count: u32,
    ) -> Result<ProcessClickResponse> {
        let request = tonic::Request::new(ProcessClickRequest {
            user_id,
            telegram_id,
            session_id,
            timestamp: chrono::Utc::now().timestamp(),
            click_count,
        });

        let response = self.client.process_click(request).await?.into_inner();

        Ok(response)
    }

    pub async fn start_session(
        &mut self,
        user_id: String,
        chat_id: i64,
        message_id: Option<i32>,
    ) -> Result<StartSessionResponse> {
        let request = tonic::Request::new(StartSessionRequest {
            user_id,
            chat_id,
            message_id: message_id.unwrap_or(0),
        });

        let response = self.client.start_session(request).await?.into_inner();

        Ok(response)
    }

    pub async fn heartbeat(&mut self, session_id: String) -> Result<HeartbeatResponse> {
        let request = tonic::Request::new(HeartbeatRequest { session_id });

        let response = self.client.heartbeat(request).await?.into_inner();

        Ok(response)
    }

    pub async fn end_session(&mut self, session_id: String) -> Result<EndSessionResponse> {
        let request = tonic::Request::new(EndSessionRequest { session_id });

        let response = self.client.end_session(request).await?.into_inner();

        Ok(response)
    }

    pub async fn get_or_create_session(
        &mut self,
        user_id: String,
        chat_id: i64,
        message_id: Option<i32>,
    ) -> Result<GetOrCreateSessionResponse> {
        let request = tonic::Request::new(GetOrCreateSessionRequest {
            user_id,
            chat_id,
            message_id: message_id.unwrap_or(0),
        });

        let response = self.client.get_or_create_session(request).await?.into_inner();

        Ok(response)
    }
}
