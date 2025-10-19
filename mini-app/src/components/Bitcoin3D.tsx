import { useRef, useState, memo } from 'react';
import { Canvas, useFrame } from '@react-three/fiber';
import { Text, Center, Environment, Float } from '@react-three/drei';
import * as THREE from 'three';

interface Bitcoin3DProps {
  onClick: () => void;
  disabled?: boolean;
}

interface FloatingText {
  id: number;
  position: [number, number, number];
}

function BitcoinMesh({ onClick, disabled, scale }: { onClick: () => void; disabled?: boolean; scale: number }) {
  const meshRef = useRef<THREE.Mesh>(null);
  const groupRef = useRef<THREE.Group>(null);
  const [hovered, setHovered] = useState(false);
  const targetScale = useRef(scale);

  useFrame((state) => {
    if (meshRef.current) {
      meshRef.current.position.y = Math.sin(state.clock.elapsedTime * 0.8) * 0.15;
    }

    if (groupRef.current) {
      const target = hovered ? scale * 1.15 : scale;
      targetScale.current += (target - targetScale.current) * 0.1;
      groupRef.current.scale.setScalar(targetScale.current);
    }
  });

  const handleClick = () => {
    if (!disabled) {
      onClick();
    }
  };

  return (
    <group ref={groupRef}>
      <mesh
        ref={meshRef}
        onClick={handleClick}
        onPointerOver={() => !disabled && setHovered(true)}
        onPointerOut={() => setHovered(false)}
        castShadow
        receiveShadow
        rotation={[Math.PI / 2, 0, 0]}
      >
        <cylinderGeometry args={[2, 2, 0.3, 64]} />
        <meshStandardMaterial
          color="#FFD700"
          metalness={0.95}
          roughness={0.1}
          emissive="#FFA500"
          emissiveIntensity={hovered ? 0.3 : 0.15}
        />
      </mesh>

      <Text
        position={[0, 0, 0.16]}
        rotation={[0, 0, 0]}
        fontSize={0.8}
        color="#ffffff"
        anchorX="center"
        anchorY="middle"
        outlineWidth={0.04}
        outlineColor="#B8860B"
      >
        ₿
      </Text>

      <Text
        position={[0, 0, -0.16]}
        rotation={[0, Math.PI, 0]}
        fontSize={0.8}
        color="#ffffff"
        anchorX="center"
        anchorY="middle"
        outlineWidth={0.04}
        outlineColor="#B8860B"
      >
        ₿
      </Text>
    </group>
  );
}

function FloatingNumber({ position, onComplete }: { position: [number, number, number]; onComplete: () => void }) {
  const ref = useRef<THREE.Group>(null);
  const [opacity, setOpacity] = useState(1);

  useFrame((_state, delta) => {
    if (ref.current) {
      ref.current.position.y += delta * 2;
      const newOpacity = opacity - delta;
      setOpacity(newOpacity);
      if (newOpacity <= 0) {
        onComplete();
      }
    }
  });

  return (
    <group ref={ref} position={position}>
      <Text
        fontSize={0.5}
        color="#FFD700"
        anchorX="center"
        anchorY="middle"
        outlineWidth={0.03}
        outlineColor="#B8860B"
      >
        +1
        <meshStandardMaterial
          color="#FFD700"
          emissive="#FFD700"
          emissiveIntensity={0.8}
          transparent
          opacity={opacity}
        />
      </Text>
    </group>
  );
}

function Scene({ onClick, disabled }: { onClick: () => void; disabled?: boolean }) {
  const [floatingNumbers, setFloatingNumbers] = useState<FloatingText[]>([]);
  const [clickScale, setClickScale] = useState(1);

  const handleClick = () => {
    onClick();

    const randomX = (Math.random() - 0.5) * 2;
    const randomZ = 2.5 + Math.random() * 0.5;

    setFloatingNumbers(prev => [
      ...prev,
      {
        id: Date.now(),
        position: [randomX, 0, randomZ]
      }
    ]);

    setClickScale(0.85);
    setTimeout(() => setClickScale(1), 150);
  };

  const removeFloatingNumber = (id: number) => {
    setFloatingNumbers(prev => prev.filter(n => n.id !== id));
  };

  return (
    <>
      <ambientLight intensity={0.4} />
      <pointLight position={[5, 5, 5]} intensity={1.5} color="#ffffff" />
      <pointLight position={[-5, 0, 5]} intensity={0.8} color="#FFD700" />
      <pointLight position={[0, -5, 2]} intensity={0.5} color="#FFA500" />

      <Environment preset="city" />

      <Float
        speed={1}
        rotationIntensity={0}
        floatIntensity={0.2}
      >
        <BitcoinMesh onClick={handleClick} disabled={disabled} scale={clickScale} />
      </Float>

      {floatingNumbers.map(num => (
        <FloatingNumber
          key={num.id}
          position={num.position}
          onComplete={() => removeFloatingNumber(num.id)}
        />
      ))}

      <mesh rotation={[-Math.PI / 2, 0, 0]} position={[0, -3, 0]} receiveShadow>
        <planeGeometry args={[15, 15]} />
        <shadowMaterial opacity={0.3} />
      </mesh>
    </>
  );
}

export const Bitcoin3D = memo(function Bitcoin3D({ onClick, disabled }: Bitcoin3DProps) {
  return (
    <div className="w-full h-96 relative bg-card/80 backdrop-blur-xl rounded-2xl overflow-hidden border border-border shadow-2xl">
      <Canvas
        camera={{
          position: [0, 0, 6],
          fov: 50,
          near: 0.1,
          far: 1000
        }}
        shadows
        gl={{
          antialias: true,
          powerPreference: 'high-performance',
          preserveDrawingBuffer: true,
          alpha: true,
        }}
        onCreated={({ gl, camera }) => {
          gl.domElement.addEventListener('webglcontextlost', (e) => {
            e.preventDefault();
            console.warn('WebGL context lost');
          });
          gl.domElement.addEventListener('webglcontextrestored', () => {
            console.log('WebGL context restored');
          });
          camera.lookAt(0, 0, 0);
        }}
        className="cursor-pointer"
      >
        <Scene onClick={onClick} disabled={disabled} />
      </Canvas>

      {disabled && (
        <div className="absolute inset-0 flex items-center justify-center bg-background/60 backdrop-blur-sm rounded-2xl">
          <div className="bg-card/80 px-6 py-3 rounded-full border border-primary/50">
            <p className="text-foreground text-lg font-semibold">Reconnecting...</p>
          </div>
        </div>
      )}
    </div>
  );
});
