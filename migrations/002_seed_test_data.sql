-- Optional test data for development
-- This file can be run separately for development/testing

-- Insert some test users (only run in development)
-- INSERT INTO users (telegram_id, username, total_clicks) VALUES
-- (123456789, 'test_user_1', 1000),
-- (987654321, 'test_user_2', 500),
-- (111222333, 'test_user_3', 250);

-- Add test clicks
-- INSERT INTO clicks (user_id, session_id, timestamp, click_count)
-- SELECT
--     u.id,
--     uuid_generate_v4(),
--     NOW() - (random() * interval '7 days'),
--     1
-- FROM users u
-- CROSS JOIN generate_series(1, 100);
