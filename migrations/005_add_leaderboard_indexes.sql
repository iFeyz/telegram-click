

CREATE INDEX IF NOT EXISTS idx_users_total_clicks_desc
ON users (total_clicks DESC)
WHERE total_clicks > 0;


CREATE INDEX IF NOT EXISTS idx_users_id_total_clicks
ON users (id, total_clicks)
WHERE total_clicks > 0;

CREATE INDEX IF NOT EXISTS idx_users_total_clicks_positive
ON users (id)
WHERE total_clicks > 0;
