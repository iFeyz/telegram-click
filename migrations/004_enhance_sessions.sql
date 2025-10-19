
ALTER TABLE sessions
  ADD COLUMN ended_at TIMESTAMP WITH TIME ZONE,
  ADD COLUMN total_clicks INT DEFAULT 0 NOT NULL;

CREATE INDEX idx_sessions_total_clicks ON sessions(total_clicks DESC);
CREATE INDEX idx_sessions_ended_at ON sessions(ended_at) WHERE ended_at IS NOT NULL;

CREATE INDEX idx_sessions_user_recent ON sessions(user_id, started_at DESC)
  WHERE is_active = TRUE;

COMMENT ON COLUMN sessions.ended_at IS 'Session end timestamp (NULL if still active)';
COMMENT ON COLUMN sessions.total_clicks IS 'Total clicks recorded during this session';
