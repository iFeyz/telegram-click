import { useState, useEffect, Suspense, lazy } from 'react';
import { useTelegram } from './hooks/useTelegram';
import { useWebSocket } from './hooks/useWebSocket';
import { Stats } from './components/Stats';
import { Loading3D } from './components/Loading3D';
import { InitialLoading3D } from './components/InitialLoading3D';
import type { LeaderboardEntry } from './types';
import './index.css';

const Bitcoin3D = lazy(() => import('./components/Bitcoin3D').then(m => ({ default: m.Bitcoin3D })));
const Leaderboard3D = lazy(() => import('./components/Leaderboard3D').then(m => ({ default: m.Leaderboard3D })));

export function App() {
  const { user, isReady, hapticFeedback } = useTelegram();
  const [totalClicks, setTotalClicks] = useState(0);
  const [isRateLimited, setIsRateLimited] = useState(false);
  const [showInitialLoading, setShowInitialLoading] = useState(true);

  const wsUrl = typeof window !== 'undefined'
    ? `${window.location.protocol === 'https:' ? 'wss:' : 'ws:'}//${window.location.host}/ws`
    : 'ws://localhost:8080/ws';

  const shouldConnect = Boolean(user && user.id > 0);

  const {
    isConnected,
    score,
    rank,
    leaderboard,
    error,
    sendClick,
    dbUsername,
    sessionStartedAt,
    isReconnection,
    isRateLimitError,
  } = useWebSocket({
    url: wsUrl,
    telegramId: user?.id || 0,
    username: user?.username || user?.firstName || 'Anonymous',
    enabled: shouldConnect,
  });


  const displayName = dbUsername || user?.username || user?.firstName || 'Anonymous';

  useEffect(() => {
    setTotalClicks(score);
  }, [score]);

  useEffect(() => {
    if (isRateLimitError) {
      console.log('Rate limit detected - disabling clicks for 1 second');
      setIsRateLimited(true);
      const timer = setTimeout(() => {
        console.log('Rate limit cleared - re-enabling clicks');
        setIsRateLimited(false);
      }, 1000);
      return () => {
        clearTimeout(timer);
      };
    } else {
      setIsRateLimited(false);
    }
  }, [isRateLimitError]);

  const handleClick = () => {
    if (isRateLimited) {
      hapticFeedback('heavy');
      return;
    }

    setTotalClicks((prev) => prev + 1);

    hapticFeedback('light');

    sendClick();
  };

  if (showInitialLoading) {
    return <InitialLoading3D onComplete={() => setShowInitialLoading(false)} />;
  }

  if (!isReady || !user) {
    return (
      <div className="flex items-center justify-center h-screen">
        <div className="text-center">
          <div className="text-4xl mb-4">⏳</div>
          <div className="text-xl">Loading...</div>
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-background text-foreground p-4">
      <div className="max-w-2xl mx-auto space-y-6">
        <div className="text-center py-4">
          <h1 className="text-5xl font-bold mb-2 bg-gradient-to-r from-primary via-accent-foreground to-primary bg-clip-text text-transparent drop-shadow-lg">
            Bitcoin Clicker
          </h1>
          <p className="text-muted-foreground">
            Welcome, <span className="font-semibold text-primary">{displayName}</span>!
          </p>
          {!isConnected && (
            <p className="text-destructive text-sm mt-2 bg-destructive/10 px-4 py-2 rounded-full inline-block border border-destructive/30">
              Reconnecting...
            </p>
          )}
          {isReconnection && isConnected && (
            <p className="text-primary text-sm mt-2 animate-pulse bg-primary/10 px-4 py-2 rounded-full inline-block border border-primary/30">
              Session resumed!
            </p>
          )}
        </div>

        <Stats
          totalClicks={totalClicks}
          globalClicks={totalClicks}
          rank={rank}
          sessionStartedAt={sessionStartedAt}
        />

        {error && !isRateLimited && !error.toLowerCase().includes('rate') && (
          <div className="bg-destructive/20 border border-destructive rounded-lg p-3 text-center">
            <p className="text-destructive-foreground">{error}</p>
          </div>
        )}

        <div className="relative">
          <Suspense fallback={<Loading3D />}>
            <Bitcoin3D onClick={handleClick} disabled={!isConnected || isRateLimited} />
          </Suspense>

          {isRateLimited && (
            <div className="absolute inset-0 flex items-center justify-center bg-background/70 backdrop-blur-sm rounded-2xl pointer-events-none">
              <div className="bg-destructive/90 px-6 py-3 rounded-full border-2 border-destructive shadow-lg animate-pulse">
                <p className="text-destructive-foreground text-lg font-bold">RATE LIMITED</p>
                <p className="text-destructive-foreground/80 text-sm">Wait 1 second...</p>
              </div>
            </div>
          )}
        </div>

        <Suspense fallback={<Loading3D />}>
          <Leaderboard3D entries={leaderboard} />
        </Suspense>
      </div>
    </div>
  );
}

export default App;
