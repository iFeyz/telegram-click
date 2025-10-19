import { useEffect, useState } from 'react';
import { motion } from 'framer-motion';
import NumberFlow from '@number-flow/react';
import { Gem, Trophy, Timer } from 'lucide-react';

interface StatsProps {
  totalClicks: number;
  globalClicks?: number;
  rank: number;
  sessionStartedAt: number | null;
}

function formatDuration(seconds: number): string {
  if (seconds < 60) {
    return `${seconds}s`;
  }
  const minutes = Math.floor(seconds / 60);
  const secs = seconds % 60;
  return `${minutes}m ${secs}s`;
}

export function Stats({ totalClicks, rank, sessionStartedAt }: StatsProps) {
  const [sessionDuration, setSessionDuration] = useState(0);

  useEffect(() => {
    if (!sessionStartedAt) return;

    const updateDuration = () => {
      const now = Math.floor(Date.now() / 1000);
      const duration = now - sessionStartedAt;
      setSessionDuration(duration);
    };

    updateDuration();
    const interval = setInterval(updateDuration, 1000);

    return () => clearInterval(interval);
  }, [sessionStartedAt]);

  const displayRank = (typeof rank === 'number' && rank > 0) ? rank : 0;

  return (
    <div className="grid grid-cols-2 gap-4">
      <motion.div
        initial={{ opacity: 0, y: 20 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ delay: 0 }}
        className="relative group col-span-2"
      >
        <div className="relative bg-card border border-border rounded-2xl p-5 transition-all duration-300 group-hover:border-primary/40 group-hover:shadow-xl group-hover:shadow-primary/10 overflow-hidden">
          <div className="absolute inset-0 bg-gradient-to-br from-primary/5 via-transparent to-transparent opacity-0 group-hover:opacity-100 transition-opacity duration-300" />

          <div className="relative z-10 flex items-center justify-between">
            <div className="flex items-center gap-4">
              <div className="inline-flex p-2.5 rounded-xl bg-primary/10 group-hover:bg-primary/20 transition-colors duration-300">
                <Timer className="w-5 h-5 text-primary" strokeWidth={2.5} />
              </div>

              <div className="text-sm text-muted-foreground font-medium">
                Session Time
              </div>
            </div>

            <div className="text-3xl font-bold text-foreground tabular-nums">
              {sessionDuration > 0 ? formatDuration(sessionDuration) : '--'}
            </div>
          </div>

          <div className="absolute bottom-0 left-0 right-0 h-1 bg-gradient-to-r from-primary/50 via-primary to-primary/50 transform scale-x-0 group-hover:scale-x-100 transition-transform duration-300 origin-left" />
        </div>
      </motion.div>

      <motion.div
        initial={{ opacity: 0, y: 20 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ delay: 0.1 }}
        className="relative group"
      >
        <div className="relative bg-card border border-border rounded-2xl p-5 transition-all duration-300 group-hover:border-primary/40 group-hover:shadow-xl group-hover:shadow-primary/10 overflow-hidden">
          <div className="absolute inset-0 bg-gradient-to-br from-primary/5 via-transparent to-transparent opacity-0 group-hover:opacity-100 transition-opacity duration-300" />

          <div className="relative z-10">
            <div className="mb-4 inline-flex p-2.5 rounded-xl bg-primary/10 group-hover:bg-primary/20 transition-colors duration-300">
              <Gem className="w-5 h-5 text-primary" strokeWidth={2.5} />
            </div>

            <div className="mb-1.5">
              <NumberFlow
                value={totalClicks}
                format={{ notation: 'standard' }}
                className="text-3xl font-bold text-foreground tabular-nums"
              />
            </div>

            <div className="text-sm text-muted-foreground font-medium">
              Your Clicks
            </div>
          </div>

          <div className="absolute bottom-0 left-0 right-0 h-1 bg-gradient-to-r from-primary/50 via-primary to-primary/50 transform scale-x-0 group-hover:scale-x-100 transition-transform duration-300 origin-left" />
        </div>
      </motion.div>

      <motion.div
        initial={{ opacity: 0, y: 20 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ delay: 0.2 }}
        className="relative group"
      >
        <div className="relative bg-card border border-border rounded-2xl p-5 transition-all duration-300 group-hover:border-primary/40 group-hover:shadow-xl group-hover:shadow-primary/10 overflow-hidden">
          <div className="absolute inset-0 bg-gradient-to-br from-primary/5 via-transparent to-transparent opacity-0 group-hover:opacity-100 transition-opacity duration-300" />

          <div className="relative z-10">
            <div className="mb-4 inline-flex p-2.5 rounded-xl bg-primary/10 group-hover:bg-primary/20 transition-colors duration-300">
              <Trophy className="w-5 h-5 text-primary" strokeWidth={2.5} />
            </div>

            <div className="mb-1.5">
              <div className="text-3xl font-bold text-foreground tabular-nums">
                #<NumberFlow
                  value={displayRank}
                  format={{ notation: 'standard' }}
                />
              </div>
            </div>

            <div className="text-sm text-muted-foreground font-medium">
              Rank
            </div>
          </div>

          <div className="absolute bottom-0 left-0 right-0 h-1 bg-gradient-to-r from-primary/50 via-primary to-primary/50 transform scale-x-0 group-hover:scale-x-100 transition-transform duration-300 origin-left" />
        </div>
      </motion.div>
    </div>
  );
}
