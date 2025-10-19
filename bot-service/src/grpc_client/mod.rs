pub mod game_client;
pub mod leaderboard_client;
pub mod pool;

pub use game_client::GameServiceClient;
pub use leaderboard_client::LeaderboardServiceClient;
pub use pool::{GrpcClientPool, get_shard_for_user};
