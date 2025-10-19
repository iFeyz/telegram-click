import { useRef, memo } from 'react';
import { Canvas, useFrame } from '@react-three/fiber';
import { Text, RoundedBox, Environment, Float, Html, OrbitControls } from '@react-three/drei';
import { motion } from 'framer-motion';
import NumberFlow from '@number-flow/react';
import { Trophy } from 'lucide-react';
import type { LeaderboardEntry } from '../types';
import * as THREE from 'three';

interface Leaderboard3DProps {
  entries: LeaderboardEntry[];
}

function AnimatedText({ children, position, fontSize, color, anchorX = "center", anchorY = "middle", maxWidth, delay = 0 }: any) {
  const textRef = useRef<any>(null);

  useFrame((state) => {
    if (textRef.current) {
      const scale = 1 + Math.sin(state.clock.elapsedTime * 2 + delay) * 0.05;
      textRef.current.scale.set(scale, scale, scale);
    }
  });

  return (
    <Text
      ref={textRef}
      position={position}
      fontSize={fontSize}
      color={color}
      anchorX={anchorX}
      anchorY={anchorY}
      maxWidth={maxWidth}
    >
      {children}
    </Text>
  );
}

function AnimatedNumber({ position, value, color }: { position: [number, number, number]; value: number; color: string }) {
  return (
    <Html position={position} center transform distanceFactor={2.5}>
      <div style={{
        color,
        fontSize: '28px',
        fontWeight: 'bold',
        textShadow: '0 0 10px rgba(0,0,0,0.5)',
        pointerEvents: 'none',
        whiteSpace: 'nowrap'
      }}>
        <NumberFlow
          value={value}
          format={{ notation: 'standard' }}
        />
      </div>
    </Html>
  );
}

function Podium({ entries }: { entries: LeaderboardEntry[] }) {
  const top3 = entries.slice(0, 3);

  const heights = [2.5, 1.8, 1.5];
  const colors = [
    '#FFD700',
    '#C0C0C0',
    '#CD7F32'
  ];
  const positions: [number, number, number][] = [
    [0, 1.25, 0],
    [-2, 0.9, 0],
    [2, 0.75, 0]
  ];

  if (top3.length === 0) return null;

  return (
    <group position={[0, -0.5, 0]}>
      {top3.map((entry, index) => {
        const actualIndex = entry.rank === 1 ? 0 : entry.rank === 2 ? 1 : 2;
        const height = heights[actualIndex] ?? 2.5;
        const color = colors[actualIndex] ?? '#FFD700';
        const position = positions[actualIndex] ?? [0, 1.25, 0] as [number, number, number];

        return (
          <Float key={entry.rank} speed={1 + index * 0.1} floatIntensity={0.1}>
            <group position={position}>
              <RoundedBox
                args={[1.5, height, 1.5]}
                radius={0.05}
                smoothness={4}
                position={[0, height / 2, 0]}
              >
                <meshStandardMaterial
                  color={color}
                  metalness={0.8}
                  roughness={0.2}
                  emissive={color}
                  emissiveIntensity={0.2}
                />
              </RoundedBox>

              <AnimatedText
                position={[0, height + 0.3, 0.76]}
                fontSize={0.3}
                color={color}
                delay={index * 0.5}
              >
                #{entry.rank}
              </AnimatedText>

              <AnimatedText
                position={[0, height + 0.7, 0.76]}
                fontSize={0.2}
                color="#ffffff"
                maxWidth={1.4}
                delay={index * 0.5 + 0.2}
              >
                {entry.username}
              </AnimatedText>

              <AnimatedNumber
                position={[0, height - 0.1, 0.76]}
                value={entry.totalClicks}
                color={color}
              />
            </group>
          </Float>
        );
      })}
    </group>
  );
}

function PodiumScene({ entries }: { entries: LeaderboardEntry[] }) {
  const groupRef = useRef<THREE.Group>(null);

  useFrame((state) => {
    if (groupRef.current) {
      groupRef.current.rotation.y = Math.sin(state.clock.elapsedTime * 0.3) * 0.1;
    }
  });

  return (
    <>
      <OrbitControls
        enableZoom={true}
        enablePan={true}
        enableRotate={true}
        minDistance={3}
        maxDistance={15}
        target={[0, 1.5, 0]}
      />

      <group ref={groupRef}>
        <ambientLight intensity={0.6} />
        <spotLight
          position={[5, 10, 5]}
          angle={0.3}
          penumbra={1}
          intensity={1}
          castShadow
        />
        <pointLight position={[-5, 5, -5]} intensity={0.5} color="#fbbf24" />

        <Environment preset="night" />

        <Podium entries={entries} />
      </group>
    </>
  );
}

function RankingCard({ entry, index }: { entry: LeaderboardEntry; index: number }) {
  return (
    <motion.div
      initial={{ opacity: 0, x: -20 }}
      animate={{ opacity: 1, x: 0 }}
      transition={{ delay: index * 0.05 }}
      className="relative group"
    >
      <div className="relative bg-card border border-border rounded-2xl p-4 transition-all duration-300 group-hover:border-primary/40 group-hover:shadow-xl group-hover:shadow-primary/10 overflow-hidden">
        <div className="absolute inset-0 bg-gradient-to-br from-primary/5 via-transparent to-transparent opacity-0 group-hover:opacity-100 transition-opacity duration-300" />

        <div className="relative z-10 flex items-center justify-between">
          <div className="flex items-center gap-4">
            <div className="inline-flex p-2.5 rounded-xl bg-primary/10 group-hover:bg-primary/20 transition-colors duration-300">
              <Trophy className="w-5 h-5 text-primary" strokeWidth={2.5} />
            </div>

            <div>
              <div className="text-xs text-muted-foreground font-medium mb-0.5">
                Rank #{entry.rank}
              </div>
              <div className="text-foreground font-semibold truncate">
                {entry.username}
              </div>
            </div>
          </div>

          <div className="ml-4">
            <NumberFlow
              value={entry.totalClicks}
              format={{ notation: 'standard' }}
              className="text-2xl font-bold text-foreground tabular-nums"
            />
          </div>
        </div>

        <div className="absolute bottom-0 left-0 right-0 h-1 bg-gradient-to-r from-primary/50 via-primary to-primary/50 transform scale-x-0 group-hover:scale-x-100 transition-transform duration-300 origin-left" />
      </div>
    </motion.div>
  );
}

export const Leaderboard3D = memo(function Leaderboard3D({ entries }: Leaderboard3DProps) {
  if (entries.length === 0) {
    return (
      <div className="w-full bg-card/80 backdrop-blur-xl rounded-2xl p-6 border border-border shadow-2xl">
        <h2 className="text-2xl font-bold text-foreground mb-6">
          <span className="bg-gradient-to-r from-primary to-accent-foreground bg-clip-text text-transparent">Leaderboard</span>
        </h2>
        <div className="text-center py-12">
          <div className="text-muted-foreground text-lg">
            Loading rankings... Click to climb the leaderboard!
          </div>
        </div>
      </div>
    );
  }

  const top3 = entries.slice(0, 3);
  const others = entries.slice(3);

  return (
    <div className="w-full space-y-6">
      <motion.h2
        initial={{ opacity: 0, y: -20 }}
        animate={{ opacity: 1, y: 0 }}
        className="text-3xl font-bold flex items-center gap-3"
      >
        <span className="bg-gradient-to-r from-primary to-accent-foreground bg-clip-text text-transparent">Leaderboard</span>
        <span className="text-sm font-normal text-muted-foreground ml-auto">
          {entries.length} players
        </span>
      </motion.h2>

      <motion.div
        initial={{ opacity: 0, scale: 0.9 }}
        animate={{ opacity: 1, scale: 1 }}
        transition={{ delay: 0.2 }}
        className="w-full h-96 rounded-2xl overflow-hidden"
      >
        <Canvas
          camera={{
            position: [0, 5, 6.8],
            fov: 60,
            near: 0.1,
            far: 1000
          }}
          shadows
          gl={{
            antialias: true,
            powerPreference: 'high-performance',
            preserveDrawingBuffer: true,
          }}
          onCreated={({ gl, camera }) => {
            gl.domElement.addEventListener('webglcontextlost', (e) => {
              e.preventDefault();
              console.warn('WebGL context lost in Leaderboard3D');
            });
            gl.domElement.addEventListener('webglcontextrestored', () => {
              console.log('WebGL context restored in Leaderboard3D');
            });
            camera.lookAt(0, 1, 0);
          }}
        >
          <PodiumScene entries={top3} />
        </Canvas>
      </motion.div>

      {others.length > 0 && (
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.4 }}
          className="space-y-3"
        >
          {others.map((entry, index) => (
            <RankingCard key={`${entry.rank}-${entry.username}`} entry={entry} index={index} />
          ))}
        </motion.div>
      )}
    </div>
  );
});
