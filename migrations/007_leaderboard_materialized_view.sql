

CREATE MATERIALIZED VIEW IF NOT EXISTS leaderboard_top_1000 AS
SELECT
    DENSE_RANK() OVER (ORDER BY total_clicks DESC) as rank,
    id::text as user_id,
    username,
    total_clicks,
    updated_at
FROM users
WHERE total_clicks > 0
ORDER BY total_clicks DESC
LIMIT 1000;



CREATE INDEX IF NOT EXISTS idx_leaderboard_mv_rank
ON leaderboard_top_1000(rank);

CREATE INDEX IF NOT EXISTS idx_leaderboard_mv_user_id
ON leaderboard_top_1000(user_id);

CREATE INDEX IF NOT EXISTS idx_leaderboard_mv_rank_username
ON leaderboard_top_1000(rank, username);


CREATE OR REPLACE FUNCTION refresh_leaderboard()
RETURNS void
LANGUAGE plpgsql
AS $$
BEGIN

    REFRESH MATERIALIZED VIEW CONCURRENTLY leaderboard_top_1000;

    RAISE NOTICE 'Leaderboard materialized view refreshed at %', NOW();
END;
$$;

GRANT EXECUTE ON FUNCTION refresh_leaderboard() TO postgres;

GRANT SELECT ON leaderboard_top_1000 TO postgres;

