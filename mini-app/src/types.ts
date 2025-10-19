/**
 * Simple types matching backend WebSocket format
 */

export interface User {
  userId: string;
  telegramId: number;
  username: string;
  totalClicks: number;
}

export interface LeaderboardEntry {
  rank: number;
  username: string;
  totalClicks: number;
}

export interface WSInitMessage {
  type: 'init';
  user_id: string;
  telegram_id: number;
  username: string;
}

export interface WSClickMessage {
  type: 'click';
  user_id: string;
  telegram_id: number;
  session_id: string;
  click_count?: number; // Number of clicks in this batch (default: 1)
}

export interface WSRefreshMessage {
  type: 'refresh';
  user_id: string;
  telegram_id: number;
}

export interface WSScoreUpdate {
  type: 'score_update';
  score: number;
  rank: number;
  user_id?: string; // UUID returned on init
  username?: string; // Database username returned on init
}

export interface WSSessionInfo {
  type: 'session_info';
  session_id: string;
  is_reconnection: boolean;
  started_at: number; // Unix timestamp
}

export interface WSLeaderboardUpdate {
  type: 'leaderboard_update';
  entries: LeaderboardEntry[];
}

export interface WSError {
  type: 'error';
  message: string;
}

export interface WSRateLimited {
  type: 'rate_limited';
  message: string;
}

export type ServerMessage = WSScoreUpdate | WSSessionInfo | WSLeaderboardUpdate | WSError | WSRateLimited;
