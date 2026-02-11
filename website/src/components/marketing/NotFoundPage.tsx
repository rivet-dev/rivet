'use client';

import { useState, useEffect, useRef } from 'react';
import { ArrowLeft } from 'lucide-react';
import { motion } from 'framer-motion';

interface NotFoundPageProps {
  thinkingImage: string;
}

const colors = [
  '#FF4500', // orange (accent)
  '#60a5fa', // blue
  '#4ade80', // green
  '#c084fc', // purple
  '#f472b6', // pink
  '#fbbf24', // yellow
  '#2dd4bf', // teal
];

const BouncingPill = () => {
  const containerRef = useRef<HTMLDivElement>(null);
  const pillRef = useRef<HTMLSpanElement>(null);
  const [position, setPosition] = useState({ x: 50, y: 50 });
  const [colorIndex, setColorIndex] = useState(0);
  const velocityRef = useRef({ x: 0.4, y: 0.25 });

  useEffect(() => {
    let animationFrameId: number;
    let lastTime = 0;
    const targetInterval = 30; // ms between updates

    const animate = (currentTime: number) => {
      if (!containerRef.current || !pillRef.current) {
        animationFrameId = requestAnimationFrame(animate);
        return;
      }

      // Throttle updates to ~30ms intervals for consistent speed
      if (currentTime - lastTime < targetInterval) {
        animationFrameId = requestAnimationFrame(animate);
        return;
      }
      lastTime = currentTime;

      const container = containerRef.current.getBoundingClientRect();
      const pill = pillRef.current.getBoundingClientRect();

      const pillWidth = (pill.width / container.width) * 100;
      const pillHeight = (pill.height / container.height) * 100;

      setPosition((prev) => {
        let newX = prev.x + velocityRef.current.x;
        let newY = prev.y + velocityRef.current.y;
        let hitEdge = false;

        if (newX <= 0 || newX >= 100 - pillWidth) {
          velocityRef.current.x *= -1;
          newX = Math.max(0, Math.min(newX, 100 - pillWidth));
          hitEdge = true;
        }

        if (newY <= 0 || newY >= 100 - pillHeight) {
          velocityRef.current.y *= -1;
          newY = Math.max(0, Math.min(newY, 100 - pillHeight));
          hitEdge = true;
        }

        if (hitEdge) {
          setColorIndex((prev) => (prev + 1) % colors.length);
        }

        return { x: newX, y: newY };
      });

      animationFrameId = requestAnimationFrame(animate);
    };

    animationFrameId = requestAnimationFrame(animate);
    return () => cancelAnimationFrame(animationFrameId);
  }, []);

  return (
    <div ref={containerRef} className="absolute inset-0 overflow-hidden">
      <span
        ref={pillRef}
        className="absolute inline-flex items-center gap-2 rounded-full bg-black/40 backdrop-blur-md border border-white/10 px-3 py-1.5 text-sm text-zinc-400 transition-colors duration-200"
        style={{
          left: `${position.x}%`,
          top: `${position.y}%`,
        }}
      >
        <span className="font-medium transition-colors duration-200" style={{ color: colors[colorIndex] }}>404</span>
        Page not found
      </span>
    </div>
  );
};

export const NotFoundPage = ({ thinkingImage }: NotFoundPageProps) => {
  return (
    <section className="relative flex min-h-screen flex-col items-center justify-center overflow-hidden px-6">
      <div className="flex flex-col items-center gap-8">
        {/* Thinking image */}
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.6 }}
          className="relative w-[280px] h-[350px] sm:w-[320px] sm:h-[400px]"
        >
          <div className="absolute inset-0 rounded-lg overflow-hidden shadow-2xl">
            <img
              src={thinkingImage}
              alt="Classical artwork depicting contemplation"
              className="w-full h-full object-cover"
            />
            <div className="absolute inset-0 bg-gradient-to-t from-black/60 via-transparent to-transparent" />
          </div>

          {/* Bouncing 404 pill */}
          <BouncingPill />
        </motion.div>

        {/* Primary button */}
        <motion.a
          href="/"
          initial={{ opacity: 0, y: 10 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.5, delay: 0.5 }}
          className="selection-dark inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md bg-white px-4 py-2 text-sm font-medium text-black transition-colors hover:bg-zinc-200"
        >
          <ArrowLeft className="h-4 w-4" />
          Back to home
        </motion.a>
      </div>
    </section>
  );
};
