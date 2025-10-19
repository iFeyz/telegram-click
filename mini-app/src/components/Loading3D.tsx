import { useRef, useEffect, useState } from 'react';
import { Canvas, useFrame } from '@react-three/fiber';
import { RoundedBox, Environment, Text } from '@react-three/drei';
import { motion } from 'framer-motion';
import * as THREE from 'three';

function LoadingBar3D() {
  const fillRef = useRef<THREE.Group>(null);
  const [progress, setProgress] = useState(0);

  useEffect(() => {
    const interval = setInterval(() => {
      setProgress((prev) => {
        if (prev >= 100) return 0;
        return prev + 2;
      });
    }, 30);

    return () => clearInterval(interval);
  }, []);

  useFrame(() => {
    if (fillRef.current) {
      const targetScale = progress / 100;
      fillRef.current.scale.x += (targetScale - fillRef.current.scale.x) * 0.1;
    }
  });

  return (
    <group>
      <Text
        position={[0, 1.5, 0]}
        fontSize={0.3}
        color="oklch(0.7686 0.1647 70.0804)"
        anchorX="center"
        anchorY="middle"
      >
        LOADING...
      </Text>

      <Text
        position={[0, -1.2, 0]}
        fontSize={0.25}
        color="oklch(0.9219 0 0)"
        anchorX="center"
        anchorY="middle"
      >
        {Math.round(progress)}%
      </Text>

      <RoundedBox
        args={[4, 0.5, 0.5]}
        radius={0.1}
        smoothness={4}
      >
        <meshStandardMaterial
          color="oklch(0.2393 0 0)"
          metalness={0.5}
          roughness={0.5}
        />
      </RoundedBox>

      <group ref={fillRef} position={[-2, 0, 0]}>
        <RoundedBox
          args={[4, 0.45, 0.45]}
          radius={0.09}
          smoothness={4}
          position={[2, 0, 0]}
        >
          <meshStandardMaterial
            color="oklch(0.7686 0.1647 70.0804)"
            metalness={0.8}
            roughness={0.2}
            emissive="oklch(0.4732 0.1247 46.2007)"
            emissiveIntensity={0.3}
          />
        </RoundedBox>
      </group>

      <pointLight
        position={[0, 0, 1]}
        intensity={1}
        distance={5}
        color="#fbbf24"
      />
    </group>
  );
}

function SpinningBitcoin() {
  const meshRef = useRef<THREE.Mesh>(null);

  useFrame((state) => {
    if (meshRef.current) {
      meshRef.current.rotation.y = state.clock.elapsedTime * 2;
      meshRef.current.position.y = Math.sin(state.clock.elapsedTime * 2) * 0.2;
    }
  });

  return (
    <mesh ref={meshRef} position={[-3.5, 0, 0]}>
      <cylinderGeometry args={[0.3, 0.3, 0.1, 32]} />
      <meshStandardMaterial
        color="oklch(0.7686 0.1647 70.0804)"
        metalness={0.9}
        roughness={0.2}
        emissive="oklch(0.4732 0.1247 46.2007)"
        emissiveIntensity={0.2}
      />
      <Text
        position={[0, 0, 0.06]}
        fontSize={0.2}
        color="oklch(0.9219 0 0)"
        anchorX="center"
        anchorY="middle"
      >
        ₿
      </Text>
    </mesh>
  );
}

function LoadingScene() {
  return (
    <>
      <ambientLight intensity={0.5} />
      <spotLight
        position={[5, 5, 5]}
        angle={0.3}
        penumbra={1}
        intensity={1}
      />
      <Environment preset="night" />

      <LoadingBar3D />
      <SpinningBitcoin />
    </>
  );
}

export function Loading3D() {
  return (
    <motion.div
      className="w-full h-96 bg-card/80 backdrop-blur-xl rounded-2xl overflow-hidden border border-border shadow-2xl"
      initial={{ opacity: 0, scale: 0.9 }}
      animate={{ opacity: 1, scale: 1 }}
      transition={{ duration: 0.3 }}
    >
      <Canvas
        camera={{ position: [0, 0, 6], fov: 50 }}
        gl={{
          antialias: true,
          powerPreference: 'high-performance',
          preserveDrawingBuffer: true,
        }}
        onCreated={({ gl }) => {
          gl.domElement.addEventListener('webglcontextlost', (e) => {
            e.preventDefault();
            console.warn('WebGL context lost in Loading3D');
          });
          gl.domElement.addEventListener('webglcontextrestored', () => {
            console.log('WebGL context restored in Loading3D');
          });
        }}
      >
        <LoadingScene />
      </Canvas>
    </motion.div>
  );
}
