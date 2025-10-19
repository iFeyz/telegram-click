
import { useEffect, useState, useCallback, useRef } from 'react';
import type { ServerMessage, LeaderboardEntry } from '../types';

interface UseWebSocketProps {
  url: string;
  telegramId: number;
  username: string;
  enabled?: boolean; // Only connect when enabled
}



export function useWebSocket({ url, telegramId, username, enabled = true }: UseWebSocketProps) {
  const [isConnected, setIsConnected] = useState(false);
  const [score, setScore] = useState(0);
  const [rank, setRank] = useState<number>(0);
  const [leaderboard, setLeaderboard] = useState<LeaderboardEntry[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [isRateLimitError, setIsRateLimitError] = useState(false);
  const [userId, setUserId] = useState<string | null>(null); // Store UUID from backend
  const [dbUsername, setDbUsername] = useState<string | null>(null); // Store database username
  const [sessionId, setSessionId] = useState<string | null>(null); // Session ID from backend
  const [sessionStartedAt, setSessionStartedAt] = useState<number | null>(null); // Session start timestamp
  const [isReconnection, setIsReconnection] = useState(false); // Whether this is a reconnection
  const wsRef = useRef<WebSocket | null>(null);
  const reconnectTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const userIdRef = useRef<string | null>(null);
  const sessionIdRef = useRef<string | null>(null);

  const pendingClicksRef = useRef<number>(0); // Accumulated clicks
  const batchIntervalRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const lastBatchSentRef = useRef<number>(0); // Timestamp of last batch sent
  const BATCH_INTERVAL_MS = 2000; // Send batch every 2 seconds
  const MIN_BATCH_INTERVAL_MS = 500; // Minimum time between batches (rate limiter)

  const connect = useCallback(() => {
    try {
      const ws = new WebSocket(url);

      ws.onopen = () => {
        console.log('WebSocket connected');
        setIsConnected(true);
        setError(null);

        ws.send(JSON.stringify({
          type: 'init',
          user_id: '', // Backend will ignore this and return actual UUID
          telegram_id: telegramId,
          username: username,
        }));
      };

      ws.onmessage = (event) => {
        try {
          const message = JSON.parse(event.data) as ServerMessage;
          console.log('WebSocket message:', message);

          switch (message.type) {
            case 'session_info':
              console.log('Session info:', message.is_reconnection ? 'Reconnected' : 'New session');
              setSessionId(message.session_id);
              sessionIdRef.current = message.session_id; // Update ref
              setSessionStartedAt(message.started_at);
              setIsReconnection(message.is_reconnection);
              if (message.is_reconnection) {
                console.log(`Reconnected to session ${message.session_id}`);
              } else {
                console.log(`Started new session ${message.session_id}`);
              }
              break;

            case 'score_update':
              console.log('Score update received - score:', message.score, 'rank:', message.rank);
              setScore(message.score ?? 0);
              if (typeof message.rank === 'number' && message.rank > 0) {
                console.log('Updating rank to:', message.rank);
                setRank(message.rank);
              } else {
                console.warn('Received invalid rank:', message.rank, '- keeping current rank');
              }
              if (message.user_id) {
                console.log('Received user_id from backend:', message.user_id);
                setUserId(message.user_id);
                userIdRef.current = message.user_id; // Update ref
              }
              if (message.username) {
                console.log('Received database username from backend:', message.username);
                setDbUsername(message.username);
              }
              setError(null);
              setIsRateLimitError(false);
              break;

            case 'leaderboard_update':
              console.log('Received leaderboard update:', message.entries.length, 'entries');
              setLeaderboard(message.entries);
              break;

            case 'error':
              console.error('Server error:', message.message);
              setError(message.message);
              setIsRateLimitError(false);
              break;

            case 'rate_limited':
              console.warn('Rate limited:', message.message);
              setError(message.message);
              setIsRateLimitError(true);
              break;
          }
        } catch (error) {
          console.error('Failed to parse WebSocket message:', error);
        }
      };

      ws.onerror = (error) => {
        console.error('WebSocket error:', error);
        setError('Connection error');
      };

      ws.onclose = () => {
        console.log('WebSocket disconnected');
        setIsConnected(false);

        reconnectTimeoutRef.current = setTimeout(() => {
          console.log('Reconnecting...');
          connect();
        }, 3000);
      };

      wsRef.current = ws;
    } catch (error) {
      console.error('Failed to connect WebSocket:', error);
      setError('Failed to connect');
    }
  }, [url, telegramId, username]);

  const sendBatch = useCallback(() => {
    const now = Date.now();

    if (now - lastBatchSentRef.current < MIN_BATCH_INTERVAL_MS) {
      console.debug('Rate limited: batch sent too recently');
      return;
    }

    const clickCount = pendingClicksRef.current;

    if (clickCount === 0) {
      return;
    }

    if (wsRef.current?.readyState === WebSocket.OPEN && userId && sessionId) {
      console.log(`Sending batch of ${clickCount} clicks`);

      pendingClicksRef.current = 0;
      lastBatchSentRef.current = now;

      wsRef.current.send(JSON.stringify({
        type: 'click',
        user_id: userId,
        telegram_id: telegramId,
        session_id: sessionId,
        click_count: clickCount,
      }));
    } else if (!userId) {
      console.warn('Cannot send batch: user_id not yet received from backend');
    } else if (!sessionId) {
      console.warn('Cannot send batch: session_id not yet received from backend');
    }
  }, [userId, telegramId, sessionId]);

  const sendClick = useCallback(() => {
    if (userId && sessionId) {
      pendingClicksRef.current += 1;
      console.debug(`Accumulated click (total pending: ${pendingClicksRef.current})`);
    } else if (!userId) {
      console.warn('Cannot accumulate click: user_id not yet received from backend');
    } else if (!sessionId) {
      console.warn('Cannot accumulate click: session_id not yet received from backend');
    }
  }, [userId, sessionId]);

  const disconnect = useCallback(() => {
    if (batchIntervalRef.current) {
      clearInterval(batchIntervalRef.current);
      batchIntervalRef.current = null;
    }

    if (pendingClicksRef.current > 0 && wsRef.current?.readyState === WebSocket.OPEN) {
      const clickCount = pendingClicksRef.current;
      const userId_local = userIdRef.current;
      const sessionId_local = sessionIdRef.current;

      if (userId_local && sessionId_local) {
        wsRef.current.send(JSON.stringify({
          type: 'click',
          user_id: userId_local,
          telegram_id: telegramId,
          session_id: sessionId_local,
          click_count: clickCount,
        }));
      }
    }

    if (reconnectTimeoutRef.current) {
      clearTimeout(reconnectTimeoutRef.current);
    }
    if (wsRef.current) {
      wsRef.current.close();
      wsRef.current = null;
    }

    pendingClicksRef.current = 0;
    lastBatchSentRef.current = 0;
  }, [telegramId]);

  useEffect(() => {
    if (enabled && telegramId > 0) {
      connect();
    }
    return () => disconnect();
  }, [enabled, telegramId, connect, disconnect]);

  useEffect(() => {
    if (userId && sessionId && isConnected) {
      console.log('Starting click batch interval (every 2 seconds)');

      if (batchIntervalRef.current) {
        clearInterval(batchIntervalRef.current);
      }

      batchIntervalRef.current = setInterval(() => {
        sendBatch();
      }, BATCH_INTERVAL_MS);

      return () => {
        if (batchIntervalRef.current) {
          clearInterval(batchIntervalRef.current);
          batchIntervalRef.current = null;
        }
      };
    }
  }, [userId, sessionId, isConnected, sendBatch]);

  return {
    isConnected,
    score,
    rank,
    leaderboard,
    error,
    sendClick,
    dbUsername, // Database username (priority over Telegram username)
    sessionStartedAt, // Session start timestamp
    isReconnection, // Whether this was a reconnection
    isRateLimitError, // Whether rate limit was exceeded
  };
}
