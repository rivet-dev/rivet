'use client';

import { useRef, useState, useEffect } from 'react';

const useCases = [
  {
    title: 'AI Agents',
    description: 'Build durable assistants with persistent, long-term memory and context.'
  },
  {
    title: 'Collaborative SaaS',
    description: 'Power real-time docs, whiteboards, and Figma-like tools with live sync.'
  },
  {
    title: 'Workflows',
    description: 'Create multi-step background jobs and workflows that can run for days.'
  },
  {
    title: 'Logistics & IoT',
    description: 'Manage state and send commands to millions of connected devices in real-time.'
  },
  {
    title: 'Live Event Platforms',
    description: 'Push real-time data, polls, and chat to 100,000+ concurrent users.'
  },
  {
    title: 'Per-User Backends',
    description: 'Instantly spin up an isolated, stateful backend for every user who signs up.'
  }
];

export function NewUseCases() {
  const scrollRef = useRef<HTMLDivElement>(null);
  const [isDragging, setIsDragging] = useState(false);
  const [startX, setStartX] = useState(0);
  const [scrollLeft, setScrollLeft] = useState(0);
  const [isPaused, setIsPaused] = useState(false);
  const animationRef = useRef<number>();

  // Auto-scroll animation
  useEffect(() => {
    const scroll = () => {
      if (!scrollRef.current || isPaused || isDragging) return;

      const container = scrollRef.current;
      const maxScroll = container.scrollWidth / 2; // Half because we have duplicates

      // Increment scroll position
      container.scrollLeft += 1;

      // Reset to start when we've scrolled through the first set
      if (container.scrollLeft >= maxScroll) {
        container.scrollLeft = 0;
      }

      animationRef.current = requestAnimationFrame(scroll);
    };

    animationRef.current = requestAnimationFrame(scroll);

    return () => {
      if (animationRef.current) {
        cancelAnimationFrame(animationRef.current);
      }
    };
  }, [isPaused, isDragging]);

  const handleMouseDown = (e: React.MouseEvent) => {
    setIsDragging(true);
    setStartX(e.pageX - (scrollRef.current?.offsetLeft || 0));
    setScrollLeft(scrollRef.current?.scrollLeft || 0);
  };

  const handleMouseMove = (e: React.MouseEvent) => {
    if (!isDragging) return;
    e.preventDefault();
    const x = e.pageX - (scrollRef.current?.offsetLeft || 0);
    const walk = (x - startX) * 2;
    if (scrollRef.current) {
      scrollRef.current.scrollLeft = scrollLeft - walk;
    }
  };

  const handleMouseUp = () => {
    setIsDragging(false);
  };

  const handleMouseLeave = () => {
    setIsDragging(false);
    setIsPaused(false);
  };

  const handleMouseEnter = () => {
    setIsPaused(true);
  };

  return (
    <section className='overflow-hidden py-24 md:py-32'>
      <h2 className='animate-on-scroll animate-fade-up text-center font-heading text-4xl font-bold tracking-tighter text-text-primary md:text-5xl'>
        From AI to IoT, Build What's Next.
      </h2>

      <div className='relative -mx-4 mt-16 px-4 [mask-image:_linear-gradient(to_right,transparent_0,_black_10%,_black_90%,transparent_100%)]'>
        <div
          ref={scrollRef}
          className='scrollbar-hide cursor-grab overflow-x-auto py-2 active:cursor-grabbing'
          onMouseDown={handleMouseDown}
          onMouseMove={handleMouseMove}
          onMouseUp={handleMouseUp}
          onMouseLeave={handleMouseLeave}
          onMouseEnter={handleMouseEnter}
          style={{ scrollbarWidth: 'none', msOverflowStyle: 'none', overflowY: 'visible' }}
        >
          <div className='flex gap-6'>
            {/* First set of cards */}
            {useCases.map((useCase, index) => (
              <div
                key={`original-${index}`}
                className='bento-box w-72 flex-none select-none rounded-xl border border-border p-6 md:w-80'
              >
                <h3 className='font-heading text-xl font-bold text-text-primary'>{useCase.title}</h3>
                <p className='mt-2 text-text-secondary'>{useCase.description}</p>
              </div>
            ))}
            {/* Duplicate set for seamless loop */}
            {useCases.map((useCase, index) => (
              <div
                key={`duplicate-${index}`}
                className='bento-box w-72 flex-none select-none rounded-xl border border-border p-6 md:w-80'
              >
                <h3 className='font-heading text-xl font-bold text-text-primary'>{useCase.title}</h3>
                <p className='mt-2 text-text-secondary'>{useCase.description}</p>
              </div>
            ))}
          </div>
        </div>
      </div>
    </section>
  );
}
