import { useRef, useState, useEffect } from 'react';
import { Canvas, useFrame } from '@react-three/fiber';
import { Sphere, MeshDistortMaterial, Environment } from '@react-three/drei';
import { motion, AnimatePresence } from 'framer-motion';
import * as THREE from 'three';

function AnimatedSphere({ position, color, speed }: { position: [number, number, number]; color: string; speed: number }) {
  const meshRef = useRef<THREE.Mesh>(null);

  useFrame((state) => {
    if (meshRef.current) {
      meshRef.current.rotation.x = state.clock.elapsedTime * speed * 1.5;
      meshRef.current.rotation.y = state.clock.elapsedTime * speed * 1.2;
      meshRef.current.rotation.z = state.clock.elapsedTime * speed * 0.8;
    }
  });

  return (
    <Sphere ref={meshRef} args={[1, 32, 32]} position={position}>
      <MeshDistortMaterial
        color={color}
        attach="material"
        distort={0.6}
        speed={speed * 2}
        roughness={0.3}
        metalness={0.7}
      />
    </Sphere>
  );
}

function LoadingScene() {
  return (
    <>
      <ambientLight intensity={0.5} />
      <pointLight position={[5, 5, 5]} intensity={1} color="#FFD700" />
      <pointLight position={[-5, -5, 5]} intensity={0.5} color="#FFA500" />
      <Environment preset="night" />

      <group position={[0, 0, 0]}>
        <AnimatedSphere position={[0, 0, 0]} color="#FFD700" speed={2.5} />
        <AnimatedSphere position={[-1.8, 0.8, -1]} color="#FFA500" speed={2} />
        <AnimatedSphere position={[1.8, -0.8, -1]} color="#FF8C00" speed={2.2} />
      </group>
    </>
  );
}

interface InitialLoading3DProps {
  onComplete: () => void;
}

export function InitialLoading3D({ onComplete }: InitialLoading3DProps) {
  const [progress, setProgress] = useState(0);
  const [isComplete, setIsComplete] = useState(false);

  useEffect(() => {
    const startTime = Date.now();
    const duration = 2000;

    const interval = setInterval(() => {
      const elapsed = Date.now() - startTime;
      const newProgress = Math.min((elapsed / duration) * 100, 100);
      setProgress(newProgress);

      if (newProgress >= 100) {
        clearInterval(interval);
        setTimeout(() => {
          setIsComplete(true);
          setTimeout(onComplete, 500);
        }, 200);
      }
    }, 16);

    return () => clearInterval(interval);
  }, [onComplete]);

  return (
    <AnimatePresence>
      {!isComplete && (
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0, scale: 1.2 }}
          transition={{ duration: 0.5 }}
          className="fixed inset-0 z-50 bg-gradient-to-br from-background via-background to-primary/20"
        >
          <div className="absolute inset-0">
            <Canvas
              camera={{
                position: [0, 0, 10],
                fov: 50,
                near: 0.1,
                far: 1000
              }}
              gl={{
                antialias: true,
                powerPreference: 'high-performance',
                alpha: true,
              }}
              onCreated={({ gl, camera }) => {
                gl.domElement.addEventListener('webglcontextlost', (e) => {
                  e.preventDefault();
                  console.warn('WebGL context lost in InitialLoading3D');
                });
                gl.domElement.addEventListener('webglcontextrestored', () => {
                  console.log('WebGL context restored in InitialLoading3D');
                });
                camera.lookAt(0, 0, 0);
              }}
            >
              <LoadingScene />
            </Canvas>
          </div>

          <div className="absolute bottom-16 left-0 right-0 flex flex-col items-center gap-3">
            <div className="text-4xl font-bold text-primary tabular-nums">
              {Math.round(progress)}%
            </div>
            <div className="w-64 h-2 bg-card/30 rounded-full overflow-hidden backdrop-blur-sm border border-primary/20">
              <motion.div
                className="h-full bg-gradient-to-r from-primary via-accent-foreground to-primary"
                initial={{ width: 0 }}
                animate={{ width: `${progress}%` }}
                transition={{ duration: 0.1 }}
              />
            </div>
          </div>

          <motion.div
            className="absolute bottom-8 left-0 right-0 text-center"
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: 0.3 }}
          >
            <p className="text-muted-foreground text-sm">
              Loading...
            </p>
          </motion.div>
        </motion.div>
      )}
    </AnimatePresence>
  );
}
