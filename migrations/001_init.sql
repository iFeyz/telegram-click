-- Create extension for UUID generation
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Users table
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    telegram_id BIGINT UNIQUE NOT NULL,
    username VARCHAR(20) NOT NULL,
    total_clicks BIGINT DEFAULT 0 NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL
);

-- Index for fast lookups
CREATE INDEX idx_users_telegram_id ON users(telegram_id);
CREATE INDEX idx_users_total_clicks ON users(total_clicks DESC);
CREATE INDEX idx_users_username ON users(username);

-- Clicks table (event sourcing)
CREATE TABLE clicks (
    id BIGSERIAL PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    session_id UUID NOT NULL,
    timestamp TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL,
    click_count INT DEFAULT 1 NOT NULL
);

-- Indexes for fast queries
CREATE INDEX idx_clicks_user_id ON clicks(user_id);
CREATE INDEX idx_clicks_session_id ON clicks(session_id);
CREATE INDEX idx_clicks_timestamp ON clicks(timestamp DESC);

-- Sessions table
CREATE TABLE sessions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    chat_id BIGINT NOT NULL,
    message_id INT,
    started_at TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL,
    last_heartbeat TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL,
    is_active BOOLEAN DEFAULT TRUE NOT NULL
);

-- Indexes for session management
CREATE INDEX idx_sessions_user_id ON sessions(user_id);
CREATE INDEX idx_sessions_active ON sessions(is_active, last_heartbeat);
CREATE INDEX idx_sessions_chat_id ON sessions(chat_id);

-- Function to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Trigger to auto-update updated_at
CREATE TRIGGER update_users_updated_at
    BEFORE UPDATE ON users
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Comments for documentation
COMMENT ON TABLE users IS 'Stores user information and total click counts';
COMMENT ON TABLE clicks IS 'Event sourcing table for all click events';
COMMENT ON TABLE sessions IS 'Tracks active user sessions for real-time updates';
COMMENT ON COLUMN users.telegram_id IS 'Telegram user ID from bot API';
COMMENT ON COLUMN users.total_clicks IS 'Denormalized total click count for performance';
COMMENT ON COLUMN sessions.last_heartbeat IS 'Last activity timestamp for session cleanup';
