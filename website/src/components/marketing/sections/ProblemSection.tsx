'use client';

import { useState, useEffect, useRef } from 'react';
import { motion } from 'framer-motion';
import { Database, Cpu, Workflow, Clock, Wifi, Zap } from 'lucide-react';

// Rivet logo icon matching the one in the hero
const RivetIcon = ({ className }: { className?: string }) => (
  <svg width="16" height="16" viewBox="0 0 176 173" className={className}>
    <g transform="translate(-32928.8,-28118.2)">
      <g transform="matrix(0.941176,0,0,0.925134,2119.4,2323.67)">
        <g clipPath="url(#_clip1)">
          <g transform="matrix(1.0625,0,0,1.08092,32936.6,27881.1)">
            <path d="M164.529,52.792L164.529,120.844C164.529,145.347 144.635,165.241 120.132,165.241L52.08,165.241C27.577,165.241 7.683,145.347 7.683,120.844L7.683,52.792C7.683,28.289 27.577,8.395 52.08,8.395L120.132,8.395C144.635,8.395 164.529,28.289 164.529,52.792Z" style={{ fill: 'none', stroke: 'currentColor', strokeWidth: '15.18px' }} />
          </g>
          <g transform="matrix(1.0625,0,0,1.08092,32737,27881.7)">
            <path d="M164.529,52.792L164.529,120.844C164.529,145.347 144.635,165.241 120.132,165.241L52.08,165.241C27.577,165.241 7.683,145.347 7.683,120.844L7.683,52.792C7.683,28.289 27.577,8.395 52.08,8.395L120.132,8.395C144.635,8.395 164.529,28.289 164.529,52.792Z" style={{ fill: 'none', stroke: 'currentColor', strokeWidth: '15.18px' }} />
          </g>
        </g>
      </g>
    </g>
    <g transform="translate(-32928.8,-28118.2)">
      <g transform="matrix(0.941176,0,0,0.925134,2119.4,2323.67)">
        <g clipPath="url(#_clip1)">
          <g transform="matrix(1.0625,0,0,1.08092,-2251.86,-2261.21)">
            <g transform="translate(32930.7,27886.2)">
              <path d="M104.323,87.121C104.584,85.628 105.665,84.411 107.117,83.977C108.568,83.542 110.14,83.965 111.178,85.069C118.49,92.847 131.296,106.469 138.034,113.637C138.984,114.647 139.343,116.076 138.983,117.415C138.623,118.754 137.595,119.811 136.267,120.208C127.471,122.841 111.466,127.633 102.67,130.266C101.342,130.664 99.903,130.345 98.867,129.425C97.83,128.504 97.344,127.112 97.582,125.747C99.274,116.055 102.488,97.637 104.323,87.121Z" style={{ fill: 'currentColor' }} />
            </g>
            <g transform="translate(32930.7,27886.2)">
              <path d="M69.264,88.242L79.739,106.385C82.629,111.392 80.912,117.803 75.905,120.694L57.762,131.168C52.755,134.059 46.344,132.341 43.453,127.335L32.979,109.192C30.088,104.185 31.806,97.774 36.813,94.883L54.956,84.408C59.962,81.518 66.374,83.236 69.264,88.242Z" style={{ fill: 'currentColor' }} />
            </g>
            <g transform="translate(32930.7,27886.2)">
              <path d="M86.541,79.464C98.111,79.464 107.49,70.084 107.49,58.514C107.49,46.944 98.111,37.565 86.541,37.565C74.971,37.565 65.591,46.944 65.591,58.514C65.591,70.084 74.971,79.464 86.541,79.464Z" style={{ fill: 'currentColor' }} />
            </g>
          </g>
        </g>
      </g>
    </g>
  </svg>
);

// Configuration for each use case diagram
// Each feature shows: Actor capability (use case specific term)
const useCaseDiagrams = {
  default: {
    title: 'Rivet Actor',
    actorLabel: 'Actor',
    actorFeatures: [
      { icon: Cpu, label: 'In-memory state' },
      { icon: Database, label: 'KV & SQLite' },
    ],
    clients: [{ label: 'Client', position: 'left' }],
    externalServices: [],
    animationSteps: 2,
  },
  'AI Agent': {
    title: 'AI Agent',
    actorLabel: 'AI Agent',
    actorFeatures: [
      { icon: Cpu, label: 'In-memory state', subtext: 'Context' },
      { icon: Database, label: 'KV & SQLite', subtext: 'Memory' },
      { icon: Clock, label: 'Scheduling', subtext: 'Tool Calls' },
    ],
    clients: [{ label: 'User', position: 'left' }],
    externalServices: [{ label: 'LLM', position: 'right' }],
    animationSteps: 4,
  },
  'Agent Memory': {
    title: 'Agent Memory',
    actorLabel: 'Agent Memory',
    actorFeatures: [
      { icon: Cpu, label: 'In-memory state', subtext: 'Context' },
      { icon: Database, label: 'KV & SQLite', subtext: 'History' },
      { icon: Clock, label: 'Sleeps when idle' },
    ],
    clients: [{ label: 'Agent', position: 'left' }],
    externalServices: [],
    animationSteps: 2,
  },
  'Game Server': {
    title: 'Game Server',
    actorLabel: 'Game Server',
    actorFeatures: [
      { icon: Cpu, label: 'In-memory state', subtext: 'Game State' },
      { icon: Wifi, label: 'WebSockets', subtext: 'Events' },
      { icon: Zap, label: 'Runs indefinitely' },
    ],
    clients: [
      { label: 'Player 1', position: 'left' },
      { label: 'Player 2', position: 'right' },
    ],
    externalServices: [],
    animationSteps: 3,
  },
  'Collaboration Backend': {
    title: 'Collaboration Backend',
    actorLabel: 'Collab Room',
    actorFeatures: [
      { icon: Cpu, label: 'In-memory state', subtext: 'Document' },
      { icon: Wifi, label: 'WebSockets', subtext: 'Sync' },
      { icon: Zap, label: 'Runs indefinitely' },
    ],
    clients: [
      { label: 'User A', position: 'left' },
      { label: 'User B', position: 'right' },
    ],
    externalServices: [],
    animationSteps: 4,
  },
  'Workflow Engine': {
    title: 'Workflow Engine',
    actorLabel: 'Workflow',
    actorFeatures: [
      { icon: Workflow, label: 'Workflows', subtext: 'Steps' },
      { icon: Clock, label: 'Scheduling', subtext: 'Retry' },
      { icon: Database, label: 'KV & SQLite', subtext: 'State' },
    ],
    clients: [{ label: 'Trigger', position: 'left' }],
    externalServices: [{ label: 'External API', position: 'right' }],
    animationSteps: 4,
  },
  'Session Store': {
    title: 'Session Store',
    actorLabel: 'Session',
    actorFeatures: [
      { icon: Database, label: 'KV & SQLite', subtext: 'User Data' },
      { icon: Cpu, label: 'In-memory state', subtext: 'Auth State' },
      { icon: Clock, label: 'Sleeps when idle' },
    ],
    clients: [{ label: 'User', position: 'left' }],
    externalServices: [],
    animationSteps: 3,
  },
  'Realtime Sync': {
    title: 'Realtime Sync',
    actorLabel: 'Sync Engine',
    actorFeatures: [
      { icon: Cpu, label: 'In-memory state', subtext: 'State' },
      { icon: Wifi, label: 'WebSockets', subtext: 'Events' },
      { icon: Zap, label: 'Runs indefinitely' },
    ],
    clients: [
      { label: 'Client 1', position: 'left' },
      { label: 'Client 2', position: 'right' },
    ],
    externalServices: [],
    animationSteps: 3,
  },
};

type UseCaseKey = keyof typeof useCaseDiagrams;

const ActorDiagram = ({ useCase = 'default' }: { useCase?: UseCaseKey }) => {
  const [activeStep, setActiveStep] = useState(0);
  const [displayedUseCase, setDisplayedUseCase] = useState<UseCaseKey>(useCase);
  const config = useCaseDiagrams[displayedUseCase] || useCaseDiagrams.default;

  // Smoothly transition to new use case without flash
  useEffect(() => {
    setDisplayedUseCase(useCase);
    setActiveStep(0);
  }, [useCase]);

  useEffect(() => {
    const interval = setInterval(() => {
      setActiveStep(prev => (prev + 1) % config.animationSteps);
    }, 1500);
    return () => clearInterval(interval);
  }, [config.animationSteps, displayedUseCase]);

  const hasRightClients = config.clients.some(c => c.position === 'right');
  const hasExternalServices = config.externalServices.length > 0;
  const leftClients = config.clients.filter(c => c.position === 'left');
  const rightClients = config.clients.filter(c => c.position === 'right');

  // Animation logic based on use case
  const getClientHighlight = (index: number, position: 'left' | 'right') => {
    if (displayedUseCase === 'Game Server' || displayedUseCase === 'Collaboration Backend' || displayedUseCase === 'Realtime Sync') {
      // Multi-client: alternate or broadcast
      if (activeStep === 0) return position === 'left' && index === 0;
      if (activeStep === 1) return true; // Actor processing
      if (activeStep === 2) return true; // Broadcast to all
      return false;
    }
    if (displayedUseCase === 'AI Agent' || displayedUseCase === 'Workflow Engine') {
      // With external service
      return activeStep === 0 && position === 'left';
    }
    // Default: simple back and forth
    return activeStep === 0;
  };

  const getActorHighlight = () => {
    if (displayedUseCase === 'AI Agent' || displayedUseCase === 'Workflow Engine') {
      return activeStep === 1 || activeStep === 3;
    }
    if (displayedUseCase === 'Game Server' || displayedUseCase === 'Collaboration Backend' || displayedUseCase === 'Realtime Sync') {
      return activeStep === 1;
    }
    return activeStep === 1;
  };

  const getExternalHighlight = () => {
    if (displayedUseCase === 'AI Agent' || displayedUseCase === 'Workflow Engine') {
      return activeStep === 2;
    }
    return false;
  };

  const getArrowHighlight = (direction: 'left' | 'right' | 'external') => {
    if (displayedUseCase === 'AI Agent') {
      if (direction === 'left') return activeStep === 0 || activeStep === 3;
      if (direction === 'external') return activeStep === 1 || activeStep === 2;
      return false;
    }
    if (displayedUseCase === 'Workflow Engine') {
      if (direction === 'left') return activeStep === 0;
      if (direction === 'external') return activeStep === 2;
      return false;
    }
    if (displayedUseCase === 'Game Server' || displayedUseCase === 'Collaboration Backend' || displayedUseCase === 'Realtime Sync') {
      if (direction === 'left') return activeStep === 0 || activeStep === 2;
      if (direction === 'right') return activeStep === 2;
      return false;
    }
    // Default
    return activeStep === 0;
  };

  return (
    <motion.div
      initial={{ opacity: 0, y: 20 }}
      whileInView={{ opacity: 1, y: 0 }}
      viewport={{ once: true }}
      transition={{ duration: 0.5 }}
      className='flex flex-col border-t border-white/10 pt-3 lg:pt-6'
    >
      <div className='mb-4 flex items-center gap-3'>
        <RivetIcon className='text-zinc-500' />
        <div className='flex items-center gap-2'>
          <h4 className='text-sm font-medium uppercase tracking-wider text-white'>Rivet Actor</h4>
          {displayedUseCase !== 'default' && (
            <motion.span
              key={`title-${displayedUseCase}`}
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              transition={{ duration: 0.2 }}
              className='text-sm font-medium uppercase tracking-wider text-zinc-500'
            >
              / {config.title}
            </motion.span>
          )}
        </div>
      </div>

      <div className='relative flex min-h-[160px] lg:h-56 items-center justify-center py-2 lg:py-0'>
        <div className='flex flex-col md:flex-row items-center gap-4 md:gap-10'>
          {/* Left Clients */}
          <div className='flex flex-row md:flex-col gap-2 md:gap-3'>
            {leftClients.map((client, idx) => (
              <motion.div
                key={`left-${idx}`}
                initial={false}
                animate={{ opacity: 1, x: 0 }}
                transition={{ duration: 0.2 }}
                className={`rounded-lg border px-3 py-1.5 md:px-6 md:py-3 ${
                  getClientHighlight(idx, 'left') ? 'border-white/20 bg-white/5 text-white' : 'border-white/5 text-zinc-500'
                } font-mono text-[10px] md:text-sm transition-colors`}
              >
                <motion.span
                  key={`${displayedUseCase}-left-${idx}`}
                  initial={{ opacity: 0 }}
                  animate={{ opacity: 1 }}
                  transition={{ duration: 0.2 }}
                >
                  {client.label}
                </motion.span>
              </motion.div>
            ))}
          </div>

          {/* Left Arrow (bidirectional) */}
          <div className='flex items-center rotate-90 md:rotate-0'>
            <div
              className={`h-1.5 w-1.5 md:h-2 md:w-2 border-l-2 border-t-2 ${
                getArrowHighlight('left') ? 'border-white/50' : 'border-white/20'
              } -rotate-45 transition-colors`}
            />
            <div
              className={`h-[2px] w-6 md:w-16 ${
                getArrowHighlight('left') ? 'bg-white/50' : 'bg-white/20'
              } transition-colors`}
            />
            <div
              className={`h-1.5 w-1.5 md:h-2 md:w-2 border-r-2 border-t-2 ${
                getArrowHighlight('left') ? 'border-white/50' : 'border-white/20'
              } rotate-45 transition-colors`}
            />
          </div>

          {/* The Actor */}
          <div className='relative'>
            <motion.div
              initial={false}
              animate={{ scale: 1 }}
              transition={{ duration: 0.2 }}
              className={`rounded-lg border px-4 py-3 md:px-8 md:py-5 ${
                getActorHighlight()
                  ? 'border-white/20 bg-white/5'
                  : 'border-white/5'
              } flex flex-col items-center gap-1.5 md:gap-2.5 transition-all`}
            >
              <div className='flex flex-col items-center'>
                <div className='font-mono text-xs md:text-base font-medium text-white'>Actor</div>
                {displayedUseCase !== 'default' && (
                  <motion.div
                    key={`subtext-${displayedUseCase}`}
                    initial={{ opacity: 0 }}
                    animate={{ opacity: 1 }}
                    transition={{ duration: 0.2 }}
                    className='font-mono text-[8px] md:text-xs text-zinc-500'
                  >
                    {config.actorLabel}
                  </motion.div>
                )}
              </div>
              <div className='h-[1px] w-full bg-white/10' />
              {config.actorFeatures.map((feature, idx) => {
                const FeatureIcon = feature.icon;
                const subtext = 'subtext' in feature ? feature.subtext : null;
                return (
                  <motion.div
                    key={`${displayedUseCase}-feature-${idx}`}
                    initial={{ opacity: 0 }}
                    animate={{ opacity: 1 }}
                    transition={{ duration: 0.2, delay: idx * 0.05 }}
                    className='flex items-center gap-1.5 md:gap-2'
                  >
                    <FeatureIcon className='h-2.5 w-2.5 md:h-4 md:w-4 text-zinc-500' />
                    <span className='text-[10px] md:text-sm text-zinc-500'>
                      {feature.label}
                      {subtext && <span className='text-zinc-600 hidden md:inline'> ({subtext})</span>}
                    </span>
                  </motion.div>
                );
              })}
            </motion.div>
          </div>

          {/* Right Arrow (if needed) */}
          {(hasRightClients || hasExternalServices) && (
            <div className='flex items-center rotate-90 md:rotate-0'>
              {!hasExternalServices && (
                <div
                  className={`h-1.5 w-1.5 md:h-2 md:w-2 border-l-2 border-t-2 ${
                    getArrowHighlight('right') ? 'border-white/50' : 'border-white/20'
                  } -rotate-45 transition-colors`}
                />
              )}
              <div
                className={`h-[2px] w-6 md:w-16 ${
                  getArrowHighlight(hasExternalServices ? 'external' : 'right') ? 'bg-white/50' : 'bg-white/20'
                } transition-colors`}
              />
              <div
                className={`h-1.5 w-1.5 md:h-2 md:w-2 border-r-2 border-t-2 ${
                  getArrowHighlight(hasExternalServices ? 'external' : 'right') ? 'border-white/50' : 'border-white/20'
                } rotate-45 transition-colors`}
              />
            </div>
          )}

          {/* Right Clients */}
          {rightClients.length > 0 && (
            <div className='flex flex-row md:flex-col gap-2 md:gap-3'>
              {rightClients.map((client, idx) => (
                <motion.div
                  key={`right-${idx}`}
                  initial={false}
                  animate={{ opacity: 1, x: 0 }}
                  transition={{ duration: 0.2 }}
                  className={`rounded-lg border px-3 py-1.5 md:px-6 md:py-3 ${
                    getClientHighlight(idx, 'right') ? 'border-white/20 bg-white/5 text-white' : 'border-white/5 text-zinc-500'
                  } font-mono text-[10px] md:text-sm transition-colors`}
                >
                  <motion.span
                    key={`${displayedUseCase}-right-${idx}`}
                    initial={{ opacity: 0 }}
                    animate={{ opacity: 1 }}
                    transition={{ duration: 0.2 }}
                  >
                    {client.label}
                  </motion.span>
                </motion.div>
              ))}
            </div>
          )}

          {/* External Services */}
          {config.externalServices.map((service, idx) => (
            <motion.div
              key={`ext-${idx}`}
              initial={false}
              animate={{ opacity: 1, x: 0 }}
              transition={{ duration: 0.2 }}
              className={`rounded-lg border px-3 py-1.5 md:px-6 md:py-3 ${
                getExternalHighlight() ? 'border-white/20 bg-white/5 text-white' : 'border-white/5 text-zinc-500'
              } font-mono text-[10px] md:text-sm transition-colors`}
            >
              <motion.span
                key={`${displayedUseCase}-ext-${idx}`}
                initial={{ opacity: 0 }}
                animate={{ opacity: 1 }}
                transition={{ duration: 0.2 }}
              >
                {service.label}
              </motion.span>
            </motion.div>
          ))}
        </div>
      </div>
    </motion.div>
  );
};

const useCaseOrder: UseCaseKey[] = [
  'AI Agent',
  'Agent Memory',
  'Game Server',
  'Collaboration Backend',
  'Workflow Engine',
  'Session Store',
  'Realtime Sync',
];

export const ProblemSection = () => {
  const [contentParallax, setContentParallax] = useState(0);
  const [activeUseCase, setActiveUseCase] = useState<UseCaseKey>('default');
  const [isInView, setIsInView] = useState(false);
  const [pillsOpacity, setPillsOpacity] = useState(0);
  const sectionRef = useRef<HTMLElement>(null);

  useEffect(() => {
    const handleScroll = () => {
      const scrollY = window.scrollY;
      const windowHeight = window.innerHeight;

      // Content parallax - content slides up to meet the hero section when "Each Actor" is centered
      const contentStart = windowHeight * 0.2;
      const contentEnd = windowHeight * 0.6;
      const maxContentOffset = 400;
      const contentOffset = Math.min(maxContentOffset, Math.max(0, (scrollY - contentStart) / (contentEnd - contentStart) * maxContentOffset));
      setContentParallax(contentOffset);
    };

    handleScroll();
    window.addEventListener('scroll', handleScroll);
    return () => {
      window.removeEventListener('scroll', handleScroll);
    };
  }, []);

  // Detect when section is in view
  useEffect(() => {
    const observer = new IntersectionObserver(
      ([entry]) => {
        setIsInView(entry.isIntersecting);
      },
      { threshold: 0.3 }
    );

    if (sectionRef.current) {
      observer.observe(sectionRef.current);
    }

    return () => observer.disconnect();
  }, []);

  // Listen for hero scroll opacity to fade in pills (inverse of hero fade out)
  useEffect(() => {
    const handleHeroScrollOpacity = (event: CustomEvent<{ opacity: number }>) => {
      // Inverse: when hero fades out (opacity -> 0), pills fade in (opacity -> 1)
      setPillsOpacity(1 - event.detail.opacity);
    };

    window.addEventListener('heroScrollOpacity', handleHeroScrollOpacity as EventListener);
    return () => window.removeEventListener('heroScrollOpacity', handleHeroScrollOpacity as EventListener);
  }, []);

  // Dispatch selected use case to hero to highlight built-in features
  useEffect(() => {
    const event = new CustomEvent('useCaseSelected', {
      detail: { useCase: activeUseCase === 'default' ? null : activeUseCase }
    });
    window.dispatchEvent(event);
  }, [activeUseCase]);

  const pills = (
    <div className='flex flex-wrap justify-center gap-2'>
      <button
        type="button"
        onClick={() => setActiveUseCase('default')}
        className={`rounded-full border px-2 py-1 text-[10px] md:px-3 md:text-xs transition-all ${
          activeUseCase === 'default'
            ? 'border-[#FF4500]/50 bg-[#FF4500]/10 text-[#FF4500]'
            : 'border-white/10 text-zinc-500 hover:border-white/20 hover:text-zinc-400'
        }`}
      >
        Actor
      </button>
      {useCaseOrder.map((useCase) => (
        <button
          key={useCase}
          type="button"
          onClick={() => setActiveUseCase(useCase)}
          className={`rounded-full border px-2 py-1 text-[10px] md:px-3 md:text-xs transition-all ${
            activeUseCase === useCase
              ? 'border-[#FF4500]/50 bg-[#FF4500]/10 text-[#FF4500]'
              : 'border-white/10 text-zinc-500 hover:border-white/20 hover:text-zinc-400'
          }`}
        >
          {useCase}
        </button>
      ))}
    </div>
  );

  return (
    <section ref={sectionRef} id='problem' className='relative border-b border-white/5 px-4 lg:px-6 pt-4 lg:pt-12 pb-12 lg:pb-48'>
      <div className='mx-auto w-full max-w-7xl'>
        {/* Mobile: simple layout */}
        <div className='lg:hidden flex flex-col gap-4'>
          <ActorDiagram useCase={activeUseCase} />
          {pills}
        </div>

        {/* Desktop: with parallax and header */}
        <div className='hidden lg:block' style={{ transform: `translateY(${400 - contentParallax}px)` }}>
          <div className='flex flex-col gap-12'>
            <motion.div
              initial={{ opacity: 0, y: 20 }}
              whileInView={{ opacity: 1, y: 0 }}
              viewport={{ once: true }}
              transition={{ duration: 0.5 }}
            >
              <p className='text-base leading-relaxed text-zinc-500'>
                An Actor is just a function. Import it like a library, write your logic, and these capabilities come built in — making Actors natively suited for agent memory, background jobs, game lobbies, and more.
              </p>
            </motion.div>

            <ActorDiagram useCase={activeUseCase} />

            <div
              className='flex justify-center mt-4 transition-opacity duration-300'
              style={{ opacity: pillsOpacity > 0 ? pillsOpacity : 1 }}
            >
              {pills}
            </div>
          </div>
        </div>
      </div>
    </section>
  );
};
