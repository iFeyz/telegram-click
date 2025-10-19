use crate::repository::LeaderboardRepository;
use std::sync::Arc;
use tonic::{Request, Response, Status};
use tracing::{debug, error, info};

pub mod game {
    tonic::include_proto!("game");
}

use game::leaderboard_service_server::LeaderboardService;
use game::{
    GetGlobalStatsRequest, GetGlobalStatsResponse, GetLeaderboardRequest, GetLeaderboardResponse,
    GetUserRankRequest, GetUserRankResponse, LeaderboardEntry, UpdateUserScoreRequest,
    UpdateUserScoreResponse,
};

#[derive(Clone)]
pub struct LeaderboardServerImpl {
    repository: Arc<LeaderboardRepository>,
}

impl LeaderboardServerImpl {
    pub fn new(repository: LeaderboardRepository) -> Self {
        Self {
            repository: Arc::new(repository),
        }
    }
}

#[tonic::async_trait]
impl LeaderboardService for LeaderboardServerImpl {
    async fn get_leaderboard(
        &self,
        request: Request<GetLeaderboardRequest>,
    ) -> Result<Response<GetLeaderboardResponse>, Status> {
        let start = std::time::Instant::now();
        let req = request.into_inner();
        let limit = if req.limit > 0 { req.limit } else { 20 };
        let offset = if req.offset > 0 { req.offset } else { 0 };

        debug!(
            "⏱️ GetLeaderboard BEGIN (CACHED): limit={}, offset={}",
            limit, offset
        );

        let repo_clone = self.repository.clone();
        let (entries_result, count_result) = tokio::join!(
            self.repository.get_leaderboard_cached(limit, offset),
            repo_clone.get_total_count()
        );

        let entries = entries_result.map_err(|e| {
            error!("Failed to get cached leaderboard: {}", e);
            Status::from(e)
        })?;

        let total_count = count_result.map_err(|e| {
            error!("Failed to get total count: {}", e);
            Status::from(e)
        })? as i32;

        let pb_entries: Vec<LeaderboardEntry> = entries
            .into_iter()
            .map(|e| LeaderboardEntry {
                rank: e.rank as i32,
                username: e.username,
                total_clicks: e.total_clicks,
                user_id: e.user_id,
            })
            .collect();

        info!(
            "⏱️ GetLeaderboard TOTAL: {:?} - Returning {} entries (total: {})",
            start.elapsed(),
            pb_entries.len(),
            total_count
        );

        Ok(Response::new(GetLeaderboardResponse {
            entries: pb_entries,
            total_count,
        }))
    }

    async fn get_user_rank(
        &self,
        request: Request<GetUserRankRequest>,
    ) -> Result<Response<GetUserRankResponse>, Status> {
        let start = std::time::Instant::now();
        let req = request.into_inner();
        let user_id = req.user_id;

        debug!("⏱️ GetUserRank BEGIN (CACHED) for user: {}", user_id);

        let result = self
            .repository
            .get_user_rank_cached(&user_id)
            .await
            .map_err(|e| {
                error!("Failed to get cached user rank for {}: {}", user_id, e);
                Status::from(e)
            })?;

        let (rank, total_clicks, found) = match result {
            Some((r, clicks)) => (r, clicks, true),
            None => (0, 0, false),
        };

        info!(
            "⏱️ GetUserRank TOTAL: {:?} - User {} rank: {}, clicks: {}, found: {}",
            start.elapsed(),
            user_id,
            rank,
            total_clicks,
            found
        );

        Ok(Response::new(GetUserRankResponse {
            rank,
            total_clicks,
            found,
        }))
    }

    async fn get_global_stats(
        &self,
        _request: Request<GetGlobalStatsRequest>,
    ) -> Result<Response<GetGlobalStatsResponse>, Status> {
        let start = std::time::Instant::now();
        debug!("⏱️ GetGlobalStats BEGIN");

        let stats = self.repository.get_global_stats().await.map_err(|e| {
            error!("Failed to get global stats: {}", e);
            Status::from(e)
        })?;

        info!(
            "⏱️ GetGlobalStats TOTAL: {:?} - clicks: {}, users: {}, sessions: {}",
            start.elapsed(),
            stats.total_clicks,
            stats.total_users,
            stats.active_sessions
        );

        Ok(Response::new(GetGlobalStatsResponse {
            total_clicks: stats.total_clicks,
            total_users: stats.total_users,
            active_sessions: stats.active_sessions,
        }))
    }

    async fn update_user_score(
        &self,
        request: Request<UpdateUserScoreRequest>,
    ) -> Result<Response<UpdateUserScoreResponse>, Status> {
        let req = request.into_inner();
        let user_id = req.user_id;
        let username = req.username;
        let score = req.score;

        debug!(
            "UpdateUserScore request: user={}, username={}, score={}",
            user_id, username, score
        );

        let new_rank = self
            .repository
            .update_score(&user_id, &username, score)
            .await
            .map_err(|e| {
                error!("Failed to update score for user {}: {}", user_id, e);
                Status::from(e)
            })?;

        info!(
            "Updated user {} score to {}, new rank: {}",
            user_id, score, new_rank
        );

        Ok(Response::new(UpdateUserScoreResponse {
            success: true,
            new_rank,
        }))
    }
}
