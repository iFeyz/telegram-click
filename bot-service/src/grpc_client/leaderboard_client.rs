use shared::errors::{Result, ServiceError};
use tonic::transport::Channel;
use std::time::Duration;

pub mod game {
    tonic::include_proto!("game");
}

use game::leaderboard_service_client::LeaderboardServiceClient as GrpcLeaderboardServiceClient;
pub use game::*;

#[derive(Clone)]
pub struct LeaderboardServiceClient {
    client: GrpcLeaderboardServiceClient<Channel>,
}

impl LeaderboardServiceClient {
    pub fn new(channel: Channel) -> Self {
        let client = GrpcLeaderboardServiceClient::new(channel);
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

        let client = GrpcLeaderboardServiceClient::connect(endpoint)
            .await
            .map_err(|e| {
                ServiceError::Grpc(format!(
                    "Failed to connect to Leaderboard Service at {}: {}",
                    url, e
                ))
            })?;

        tracing::info!("Connected to Leaderboard Service at {}", url);

        Ok(Self { client })
    }

    pub async fn get_leaderboard(
        &mut self,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> Result<GetLeaderboardResponse> {
        let request = tonic::Request::new(GetLeaderboardRequest {
            limit: limit.unwrap_or(20),
            offset: offset.unwrap_or(0),
        });

        let response = self
            .client
            .get_leaderboard(request)
            .await?
            .into_inner();

        Ok(response)
    }

    pub async fn get_user_rank(&mut self, user_id: String) -> Result<GetUserRankResponse> {
        let request = tonic::Request::new(GetUserRankRequest { user_id });

        let response = self.client.get_user_rank(request).await?.into_inner();

        Ok(response)
    }

    pub async fn get_global_stats(&mut self) -> Result<GetGlobalStatsResponse> {
        let request = tonic::Request::new(GetGlobalStatsRequest {});

        let response = self
            .client
            .get_global_stats(request)
            .await?
            .into_inner();

        Ok(response)
    }
}
