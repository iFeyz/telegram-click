
DROP INDEX IF EXISTS idx_leaderboard_mv_user_id;


CREATE UNIQUE INDEX idx_leaderboard_mv_user_id ON leaderboard_top_1000(user_id);

