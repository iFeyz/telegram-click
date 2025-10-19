use tonic::{Request, Response, Status};
use shared::proto::{
    game_service_server::GameService,
    CreateUserRequest, CreateUserResponse,
    GetUserRequest, GetUserResponse,
    UpdateUsernameRequest, UpdateUsernameResponse,
    ProcessClickRequest, ProcessClickResponse,
    StartSessionRequest, StartSessionResponse,
    HeartbeatRequest, HeartbeatResponse,
    EndSessionRequest, EndSessionResponse,
    GetSessionStatsRequest, GetSessionStatsResponse,
    GetOrCreateSessionRequest, GetOrCreateSessionResponse,
};
use shared::{UserId, SessionId};

use crate::service::{UserService, ClickService, SessionService};


pub struct GameServerImpl {
    user_service: UserService,
    click_service: ClickService,
    session_service: SessionService,
}

impl GameServerImpl {

    pub fn new(
        user_service: UserService,
        click_service: ClickService,
        session_service: SessionService,
    ) -> Self {
        Self {
            user_service,
            click_service,
            session_service,
        }
    }
}

#[tonic::async_trait]
impl GameService for GameServerImpl {
    async fn create_user(
        &self,
        request: Request<CreateUserRequest>,
    ) -> Result<Response<CreateUserResponse>, Status> {
        let req = request.into_inner();

        tracing::debug!(
            telegram_id = req.telegram_id,
            username = req.username,
            "CreateUser request"
        );

        match self.user_service.register_user(req.telegram_id, &req.username).await {
            Ok(user) => {
                let response = CreateUserResponse {
                    user_id: user.id.to_string(),
                    username: user.username.as_str().to_string(),
                    total_clicks: user.total_clicks,
                    success: true,
                    message: "User created successfully".to_string(),
                };
                Ok(Response::new(response))
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to create user");
                Err(e.into())
            }
        }
    }

    async fn get_user(
        &self,
        request: Request<GetUserRequest>,
    ) -> Result<Response<GetUserResponse>, Status> {
        let req = request.into_inner();

        tracing::debug!(telegram_id = req.telegram_id, "GetUser request");

        match self.user_service.get_user(req.telegram_id).await {
            Ok(user) => {
                let response = GetUserResponse {
                    user_id: user.id.to_string(),
                    telegram_id: user.telegram_id,
                    username: user.username.as_str().to_string(),
                    total_clicks: user.total_clicks,
                    exists: true,
                };
                Ok(Response::new(response))
            }
            Err(shared::ServiceError::UserNotFound(_)) => {
                let response = GetUserResponse {
                    user_id: String::new(),
                    telegram_id: req.telegram_id,
                    username: String::new(),
                    total_clicks: 0,
                    exists: false,
                };
                Ok(Response::new(response))
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to get user");
                Err(e.into())
            }
        }
    }

    async fn update_username(
        &self,
        request: Request<UpdateUsernameRequest>,
    ) -> Result<Response<UpdateUsernameResponse>, Status> {
        let req = request.into_inner();

        tracing::debug!(
            user_id = req.user_id,
            new_username = req.new_username,
            "UpdateUsername request"
        );

        let user_id = UserId::from_string(&req.user_id)
            .map_err(|e| Status::invalid_argument(e.to_string()))?;

        match self.user_service.change_username(&user_id, &req.new_username).await {
            Ok(_) => {
                let response = UpdateUsernameResponse {
                    success: true,
                    message: "Username updated successfully".to_string(),
                    username: req.new_username,
                };
                Ok(Response::new(response))
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to update username");
                Err(e.into())
            }
        }
    }

    async fn process_click(
        &self,
        request: Request<ProcessClickRequest>,
    ) -> Result<Response<ProcessClickResponse>, Status> {
        let req = request.into_inner();
        let click_count = if req.click_count == 0 { 1 } else { req.click_count };

        let user_id = UserId::from_string(&req.user_id)
            .map_err(|e| Status::invalid_argument(e.to_string()))?;

        let session_id = SessionId::from_string(&req.session_id)
            .map_err(|e| Status::invalid_argument(e.to_string()))?;

        // Get user to retrieve username for batch accumulation
        let user = match self.user_service.get_user_by_id(&user_id).await {
            Ok(user) => user,
            Err(_) => {
                return Err(Status::not_found("User not found"));
            }
        };

        match self.click_service.process_click(&user_id, user.username.as_str(), &session_id, click_count).await {
            Ok(click_result) => {
                let current_rank = 0;

                let response = ProcessClickResponse {
                    new_total: click_result.total_clicks,
                    current_rank,
                    rate_limited: false,
                    message: "Click processed".to_string(),
                    success: true,
                    session_clicks: 0, // Deprecated - no longer tracked
                };
                Ok(Response::new(response))
            }
            Err(shared::ServiceError::RateLimitExceeded) => {
                let response = ProcessClickResponse {
                    new_total: 0,
                    current_rank: 0,
                    rate_limited: true,
                    message: "Rate limit exceeded".to_string(),
                    success: false,
                    session_clicks: 0,
                };
                Ok(Response::new(response))
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to process click");
                Err(e.into())
            }
        }
    }

    async fn start_session(
        &self,
        request: Request<StartSessionRequest>,
    ) -> Result<Response<StartSessionResponse>, Status> {
        let req = request.into_inner();

        let user_id = UserId::from_string(&req.user_id)
            .map_err(|e| Status::invalid_argument(e.to_string()))?;

        let message_id = if req.message_id == 0 {
            None
        } else {
            Some(req.message_id)
        };

        match self.session_service.start_session(&user_id, req.chat_id, message_id).await {
            Ok(session) => {
                let response = StartSessionResponse {
                    session_id: session.id.to_string(),
                    success: true,
                    total_clicks: 0,
                    started_at: session.started_at.timestamp(),
                };
                Ok(Response::new(response))
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to start session");
                Err(e.into())
            }
        }
    }

    async fn heartbeat(
        &self,
        request: Request<HeartbeatRequest>,
    ) -> Result<Response<HeartbeatResponse>, Status> {
        let req = request.into_inner();

        let session_id = SessionId::from_string(&req.session_id)
            .map_err(|e| Status::invalid_argument(e.to_string()))?;

        match self.session_service.heartbeat(&session_id).await {
            Ok(_) => {
                let response = HeartbeatResponse { active: true };
                Ok(Response::new(response))
            }
            Err(_) => {
                let response = HeartbeatResponse { active: false };
                Ok(Response::new(response))
            }
        }
    }

    async fn end_session(
        &self,
        request: Request<EndSessionRequest>,
    ) -> Result<Response<EndSessionResponse>, Status> {
        let req = request.into_inner();

        let session_id = SessionId::from_string(&req.session_id)
            .map_err(|e| Status::invalid_argument(e.to_string()))?;

        match self.session_service.end_session(&session_id).await {
            Ok(_) => {
                let response = EndSessionResponse { success: true };
                Ok(Response::new(response))
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to end session");
                Err(e.into())
            }
        }
    }

    async fn get_session_stats(
        &self,
        request: Request<GetSessionStatsRequest>,
    ) -> Result<Response<GetSessionStatsResponse>, Status> {
        let req = request.into_inner();

        let session_id = SessionId::from_string(&req.session_id)
            .map_err(|e| Status::invalid_argument(e.to_string()))?;

        match self.session_service.get_stats(&session_id).await {
            Ok(stats) => {
                let response = GetSessionStatsResponse {
                    session_id: stats.session_id.to_string(),
                    user_id: stats.user_id.to_string(),
                    chat_id: stats.chat_id,
                    message_id: stats.message_id.unwrap_or(0),
                    started_at: stats.started_at.timestamp(),
                    ended_at: stats.ended_at.map(|t| t.timestamp()).unwrap_or(0),
                    last_heartbeat: stats.last_heartbeat.timestamp(),
                    total_clicks: stats.total_clicks,
                    is_active: stats.is_active,
                    duration_secs: stats.duration_secs,
                };
                Ok(Response::new(response))
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to get session stats");
                Err(e.into())
            }
        }
    }

    async fn get_or_create_session(
        &self,
        request: Request<GetOrCreateSessionRequest>,
    ) -> Result<Response<GetOrCreateSessionResponse>, Status> {
        let req = request.into_inner();

        let user_id = UserId::from_string(&req.user_id)
            .map_err(|e| Status::invalid_argument(e.to_string()))?;

        let message_id = if req.message_id == 0 {
            None
        } else {
            Some(req.message_id)
        };

        match self.session_service.get_or_create_session(&user_id, req.chat_id, message_id).await {
            Ok((stats, is_reconnection)) => {
                let response = GetOrCreateSessionResponse {
                    session_id: stats.session_id.to_string(),
                    success: true,
                    is_reconnection,
                    total_clicks: stats.total_clicks,
                    started_at: stats.started_at.timestamp(),
                    duration_secs: stats.duration_secs,
                };
                Ok(Response::new(response))
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to get or create session");
                Err(e.into())
            }
        }
    }
}
