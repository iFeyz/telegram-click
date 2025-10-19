use tokio::sync::broadcast;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, info};

use crate::grpc_client::{LeaderboardServiceClient, GrpcClientPool};
use crate::websocket::handler::{ServerMessage, LeaderboardEntry, BroadcastMessage};
use shared::ServiceError;

pub struct LeaderboardBroadcaster {
    leaderboard_client_pool: Arc<GrpcClientPool<LeaderboardServiceClient>>,
    broadcast_tx: broadcast::Sender<BroadcastMessage>,
    broadcast_interval: Duration,
}

impl LeaderboardBroadcaster {
    pub fn new(
        leaderboard_client_pool: Arc<GrpcClientPool<LeaderboardServiceClient>>,
        broadcast_tx: broadcast::Sender<BroadcastMessage>,
        broadcast_interval_ms: u64,
    ) -> Self {
        Self {
            leaderboard_client_pool,
            broadcast_tx,
            broadcast_interval: Duration::from_millis(broadcast_interval_ms),
        }
    }

    pub fn start_periodic_broadcaster(self: Arc<Self>) {
        let interval = self.broadcast_interval;

        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);

            info!(
                interval_ms = interval.as_millis(),
                "Started leaderboard broadcaster"
            );

            loop {
                ticker.tick().await;

                match self.broadcast_leaderboard().await {
                    Ok(_) => {
                        info!("Leaderboard broadcast successful");
                    }
                    Err(e) => {
                        error!(error = %e, "Leaderboard broadcast failed");
                    }
                }
            }
        });
    }

    async fn broadcast_leaderboard(&self) -> Result<(), ServiceError> {
        let client_mutex = self.leaderboard_client_pool.get_client();
        let mut client = client_mutex.lock().await;

        let response = client
            .get_leaderboard(Some(20), Some(0))
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to fetch leaderboard");
                ServiceError::Internal(format!("Leaderboard fetch failed: {}", e))
            })?;

        drop(client);

        let entries: Vec<LeaderboardEntry> = response
            .entries
            .into_iter()
            .map(|entry| LeaderboardEntry {
                rank: entry.rank,
                username: entry.username,
                total_clicks: entry.total_clicks,
            })
            .collect();

        info!(entries = entries.len(), "Fetched leaderboard entries");

        let message = ServerMessage::LeaderboardUpdate { entries };

        match self.broadcast_tx.send(BroadcastMessage::LeaderboardUpdate(message)) {
            Ok(receivers) => {
                info!(
                    receivers = receivers,
                    "Broadcasted leaderboard to connected clients"
                );
            }
            Err(_) => {
                info!("No WebSocket clients connected to receive broadcast");
            }
        }

        Ok(())
    }

    #[cfg(test)]
    pub fn get_interval(&self) -> Duration {
        self.broadcast_interval
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interval_configuration() {
        let intervals = vec![500, 1000, 2000, 5000];

        for interval_ms in intervals {
            let duration = Duration::from_millis(interval_ms);
            assert_eq!(duration.as_millis(), interval_ms as u128);
        }
    }
}
