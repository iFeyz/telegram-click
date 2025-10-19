

DROP TABLE IF EXISTS clicks;

COMMENT ON TABLE users IS 'Stores user information and total click counts (batch updated)';
COMMENT ON COLUMN users.total_clicks IS 'Total click count updated via batch processing';
