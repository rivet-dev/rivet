'use client';

import { useState, useEffect } from 'react';
import { Terminal, ArrowRight, Check, Database, HardDrive, GitBranch, Clock, Wifi, Infinity, Moon } from 'lucide-react';
import { motion, AnimatePresence } from 'framer-motion';

const ThinkingImageCycler = ({ images }: { images: string[] }) => {
  const [currentIndex, setCurrentIndex] = useState(0);
  const [showFan, setShowFan] = useState(false);

  const handleClick = () => {
    setShowFan(false);
    setCurrentIndex((prev) => (prev + 1) % images.length);
  };

  const handleMouseEnter = () => {
    setShowFan(true);
    setTimeout(() => {
      setShowFan(false);
    }, 1000);
  };

  const handleMouseLeave = () => {
    setShowFan(false);
  };

  // Get indices for the fanned cards behind the main one
  const getNextIndices = (count: number) => {
    const indices = [];
    for (let i = 1; i <= count; i++) {
      indices.push((currentIndex + i) % images.length);
    }
    return indices;
  };

  const fanCards = getNextIndices(3);

  return (
    <div
      className="relative w-[280px] h-[350px] sm:w-[400px] sm:h-[500px] cursor-pointer"
      onClick={handleClick}
      onMouseEnter={handleMouseEnter}
      onMouseLeave={handleMouseLeave}
    >
      {/* Fanned cards behind */}
      {fanCards.map((imageIndex, i) => {
        const rotation = showFan ? (i + 1) * 6 : 0;
        const translateX = showFan ? (i + 1) * 15 : 0;
        const translateY = showFan ? (i + 1) * -5 : 0;
        const scale = 1 - (i + 1) * 0.02;

        return (
          <div
            key={`fan-${i}`}
            className="absolute inset-0 rounded-lg overflow-hidden shadow-xl transition-all duration-300 ease-out"
            style={{
              transform: `rotate(${rotation}deg) translateX(${translateX}px) translateY(${translateY}px) scale(${scale})`,
              zIndex: 3 - i,
              opacity: showFan ? 0.8 - i * 0.2 : 0,
            }}
          >
            <img
              src={images[imageIndex]}
              alt="Classical artwork depicting contemplation"
              loading="lazy"
              decoding="async"
              className="w-full h-full object-cover select-none pointer-events-none"
            />
          </div>
        );
      })}

      {/* Main card */}
      <div
        className="absolute inset-0 rounded-lg overflow-hidden shadow-2xl transition-transform duration-300 ease-out"
        style={{
          zIndex: 10,
          transform: showFan ? 'rotate(-3deg) translateX(-10px)' : 'rotate(0deg) translateX(0px)',
        }}
      >
        {images.map((src, index) => (
          <img
            key={src}
            src={src}
            alt="Classical artwork depicting contemplation and deep thought"
            loading={index === 0 ? 'eager' : 'lazy'}
            decoding="async"
            className={`absolute inset-0 w-full h-full object-cover transition-opacity duration-300 select-none pointer-events-none ${
              index === currentIndex ? 'opacity-100' : 'opacity-0'
            }`}
          />
        ))}
        <div className="absolute inset-0 bg-gradient-to-t from-black/60 via-transparent to-transparent" />
      </div>
    </div>
  );
};

const ActorsLogoWithIcon = ({ hoveredFeature }: { hoveredFeature: string | null }) => {
  const iconMap: Record<string, typeof Database> = {
    'In-memory state': Database,
    'KV & SQLite': HardDrive,
    'Workflows': GitBranch,
    'Scheduling': Clock,
    'WebSockets': Wifi,
    'Runs indefinitely': Infinity,
    'Sleeps when idle': Moon,
  };

  const Icon = hoveredFeature ? iconMap[hoveredFeature] : null;

  return (
    <svg width="24" height="24" viewBox="0 0 176 173" className="inline-block">
      {/* Outline - two overlapping rounded rectangles */}
      <g transform="translate(-32928.8,-28118.2)">
        <g transform="matrix(0.941176,0,0,0.925134,2119.4,2323.67)">
          <g clipPath="url(#_clip1)">
            <g transform="matrix(1.0625,0,0,1.08092,32936.6,27881.1)">
              <path d="M164.529,52.792L164.529,120.844C164.529,145.347 144.635,165.241 120.132,165.241L52.08,165.241C27.577,165.241 7.683,145.347 7.683,120.844L7.683,52.792C7.683,28.289 27.577,8.395 52.08,8.395L120.132,8.395C144.635,8.395 164.529,28.289 164.529,52.792Z" style={{ fill: 'none', stroke: 'white', strokeWidth: '15.18px' }} />
            </g>
            <g transform="matrix(1.0625,0,0,1.08092,32737,27881.7)">
              <path d="M164.529,52.792L164.529,120.844C164.529,145.347 144.635,165.241 120.132,165.241L52.08,165.241C27.577,165.241 7.683,145.347 7.683,120.844L7.683,52.792C7.683,28.289 27.577,8.395 52.08,8.395L120.132,8.395C144.635,8.395 164.529,28.289 164.529,52.792Z" style={{ fill: 'none', stroke: 'white', strokeWidth: '15.18px' }} />
            </g>
          </g>
        </g>
      </g>

      {/* Inner content - either default shapes or hovered icon */}
      <AnimatePresence mode="wait" initial={false}>
        {!Icon ? (
          // Default inner shapes
          <motion.g
            key="default"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            transition={{ duration: 0.15 }}
            transform="translate(-32928.8,-28118.2)"
          >
            <g transform="matrix(0.941176,0,0,0.925134,2119.4,2323.67)">
              <g clipPath="url(#_clip1)">
                <g transform="matrix(1.0625,0,0,1.08092,-2251.86,-2261.21)">
                  <g transform="translate(32930.7,27886.2)">
                    <path d="M104.323,87.121C104.584,85.628 105.665,84.411 107.117,83.977C108.568,83.542 110.14,83.965 111.178,85.069C118.49,92.847 131.296,106.469 138.034,113.637C138.984,114.647 139.343,116.076 138.983,117.415C138.623,118.754 137.595,119.811 136.267,120.208C127.471,122.841 111.466,127.633 102.67,130.266C101.342,130.664 99.903,130.345 98.867,129.425C97.83,128.504 97.344,127.112 97.582,125.747C99.274,116.055 102.488,97.637 104.323,87.121Z" style={{ fill: 'white' }} />
                  </g>
                  <g transform="translate(32930.7,27886.2)">
                    <path d="M69.264,88.242L79.739,106.385C82.629,111.392 80.912,117.803 75.905,120.694L57.762,131.168C52.755,134.059 46.344,132.341 43.453,127.335L32.979,109.192C30.088,104.185 31.806,97.774 36.813,94.883L54.956,84.408C59.962,81.518 66.374,83.236 69.264,88.242Z" style={{ fill: 'white' }} />
                  </g>
                  <g transform="translate(32930.7,27886.2)">
                    <path d="M86.541,79.464C98.111,79.464 107.49,70.084 107.49,58.514C107.49,46.944 98.111,37.565 86.541,37.565C74.971,37.565 65.591,46.944 65.591,58.514C65.591,70.084 74.971,79.464 86.541,79.464Z" style={{ fill: 'white' }} />
                  </g>
                </g>
              </g>
            </g>
          </motion.g>
        ) : (
          // Hovered icon centered in the logo
          <motion.foreignObject
            key={hoveredFeature}
            x="20"
            y="20"
            width="136"
            height="133"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            transition={{ duration: 0.15 }}
          >
            <div className="flex h-full w-full items-center justify-center">
              <Icon className="h-20 w-20 text-white" strokeWidth={2} />
            </div>
          </motion.foreignObject>
        )}
      </AnimatePresence>
    </svg>
  );
};

const CopyInstallButton = () => {
  const [copied, setCopied] = useState(false);

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText('npx skills add rivet-dev/skills');
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch (err) {
      console.error('Failed to copy:', err);
    }
  };

  return (
    <div className='relative group w-full sm:w-auto'>
      <button
        onClick={handleCopy}
        className='w-full sm:w-auto inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md border border-white/10 px-4 py-2 text-sm text-zinc-300 transition-colors hover:border-white/20 hover:text-white'
      >
        {copied ? <Check className='h-4 w-4 text-green-500' /> : <Terminal className='h-4 w-4' />}
        npx skills add rivet-dev/skills
      </button>
      <div className='absolute left-1/2 -translate-x-1/2 top-full mt-4 opacity-0 translate-y-2 group-hover:opacity-100 group-hover:translate-y-0 transition-all duration-200 ease-out text-xs text-zinc-500 whitespace-nowrap pointer-events-none font-mono'>
        Give this to your coding agent
      </div>
    </div>
  );
};

interface RedesignedHeroProps {
  latestChangelogTitle: string;
  thinkingImages: string[];
}

const useCases = ['AI Agent', 'Agent Memory', 'Game Server', 'Collaboration Backend', 'Workflow Engine', 'Session Store', 'Realtime Sync'];

const featureToUseCases: Record<string, string[]> = {
  'In-memory state': ['AI Agent', 'Agent Memory', 'Game Server', 'Session Store', 'Realtime Sync'],
  'KV & SQLite': ['Agent Memory', 'Session Store', 'Workflow Engine'],
  'Workflows': ['Workflow Engine', 'AI Agent'],
  'Scheduling': ['Workflow Engine', 'AI Agent'],
  'WebSockets': ['Game Server', 'Collaboration Backend', 'Realtime Sync'],
  'Runs indefinitely': ['Game Server', 'AI Agent', 'Collaboration Backend', 'Realtime Sync'],
  'Sleeps when idle': ['Workflow Engine', 'Agent Memory', 'Session Store'],
};

export const RedesignedHero = ({ latestChangelogTitle, thinkingImages }: RedesignedHeroProps) => {
  const [hoveredFeature, setHoveredFeature] = useState<string | null>(null);
  const [hoveredUseCase, setHoveredUseCase] = useState<string | null>(null);
  const [carouselUseCase, setCarouselUseCase] = useState<string | null>(null);
  const [scrollOpacity, setScrollOpacity] = useState(1);
  const features = ['In-memory state', 'KV & SQLite', 'Workflows', 'Scheduling', 'WebSockets', 'Runs indefinitely', 'Sleeps when idle'];

  // Use hovered use case if hovering, otherwise use carousel use case
  const activeUseCase = hoveredUseCase || carouselUseCase;

  const highlightedUseCases = hoveredFeature ? featureToUseCases[hoveredFeature] || [] : [];

  const highlightedFeatures = activeUseCase
    ? features.filter(feature => featureToUseCases[feature]?.includes(activeUseCase))
    : [];

  useEffect(() => {
    const handleScroll = () => {
      const scrollY = window.scrollY;
      const windowHeight = window.innerHeight;
      const isMobile = window.innerWidth < 1024;

      // Mobile: no fade/blur effect
      if (isMobile) {
        setScrollOpacity(1);
        return;
      }

      // Desktop: fade starts at 20%, done at 60% (for main content only, not actor sections)
      const mainFadeStart = windowHeight * 0.2;
      const mainFadeEnd = windowHeight * 0.6;
      const mainOpacity = 1 - Math.min(1, Math.max(0, (scrollY - mainFadeStart) / (mainFadeEnd - mainFadeStart)));
      setScrollOpacity(mainOpacity);
    };

    window.addEventListener('scroll', handleScroll);
    return () => window.removeEventListener('scroll', handleScroll);
  }, []);

  // Listen for carousel rotation events from ProblemSection
  useEffect(() => {
    const handleCarouselRotate = (event: CustomEvent<{ useCase: string | null }>) => {
      setCarouselUseCase(event.detail.useCase);
    };

    window.addEventListener('carouselRotate', handleCarouselRotate as EventListener);
    return () => window.removeEventListener('carouselRotate', handleCarouselRotate as EventListener);
  }, []);

  // Dispatch custom event when use case hover changes
  useEffect(() => {
    const event = new CustomEvent('useCaseHover', {
      detail: { useCase: hoveredUseCase }
    });
    window.dispatchEvent(event);
  }, [hoveredUseCase]);

  return (
    <section className='relative flex min-h-screen flex-col justify-between'>
      {/* Centered content */}
      <div className='flex flex-col justify-start pt-32 lg:justify-center lg:pt-0 lg:pb-20 lg:flex-1 px-6' style={{ opacity: scrollOpacity, filter: `blur(${(1 - scrollOpacity) * 8}px)` }}>
        <div className='mx-auto w-full max-w-7xl'>
          <div className='flex flex-col gap-12 lg:flex-row lg:items-center lg:justify-between lg:gap-32 xl:gap-48 2xl:gap-64'>
            <div className='max-w-xl'>
              {/* Mobile changelog link - above title */}
              <motion.div
                initial={{ opacity: 0 }}
                animate={{ opacity: 1 }}
                transition={{ duration: 0.5, delay: 0.15 }}
                className='mb-6 lg:hidden'
              >
                <a
                  href='/changelog'
                  className='inline-flex items-center gap-2 rounded-full bg-black/60 backdrop-blur-md border border-white/10 px-3 py-1.5 text-xs text-zinc-300 transition-colors hover:border-white/20 hover:text-white'
                >
                  <span className='h-[4px] w-[4px] rounded-full bg-[#ff6030]' style={{ boxShadow: '0 0 2px #ffaa60, 0 0 4px #ff8040, 0 0 10px #ff6020, 0 0 20px rgba(255, 69, 0, 0.8)' }} />
                  {latestChangelogTitle}
                  <ArrowRight className='h-3 w-3' />
                </a>
              </motion.div>

              <motion.h1
                initial={{ opacity: 0, y: 20 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ duration: 0.5 }}
                className='mb-4 text-4xl font-normal leading-[1.1] tracking-tight text-white md:text-6xl'
              >
                The primitive for <br />
                software that thinks.
              </motion.h1>

              <motion.p
                initial={{ opacity: 0, y: 20 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ duration: 0.5, delay: 0.05 }}
                className='mb-6 text-lg text-zinc-400 md:text-xl'
              >
                Rivet Actors are a serverless primitive for stateful workloads.
              </motion.p>

              <motion.div
                initial={{ opacity: 0, y: 20 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ duration: 0.5, delay: 0.1 }}
                className='flex flex-col gap-3 sm:flex-row'
              >
                <a href='/docs'
                  className='selection-dark inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md bg-white px-4 py-2 text-sm font-medium text-black transition-colors hover:bg-zinc-200'
                >
                  Start Building
                  <ArrowRight className='h-4 w-4' />
                </a>
                <CopyInstallButton />
              </motion.div>
            </div>

            {/* Right side - Cycling thinking images */}
            <motion.div
              initial={{ opacity: 0, x: 20 }}
              animate={{ opacity: 1, x: 0 }}
              transition={{ duration: 0.8, delay: 0.2 }}
              className='flex-shrink-0 hidden lg:block'
            >
              <ThinkingImageCycler images={thinkingImages} />
            </motion.div>
          </div>

          {/* Mobile: Image only */}
          <div className='lg:hidden mt-12'>
            <motion.div
              initial={{ opacity: 0, x: 20 }}
              animate={{ opacity: 1, x: 0 }}
              transition={{ duration: 0.8, delay: 0.2 }}
              className='flex justify-center'
            >
              <ThinkingImageCycler images={thinkingImages} />
            </motion.div>
          </div>
        </div>
      </div>

      {/* Mobile: Actor sections */}
      <div className='lg:hidden px-6 mt-12 pb-6'>
        <div className='mx-auto w-full max-w-7xl'>
          <div className='flex flex-col gap-4'>
            <motion.div
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ duration: 0.5, delay: 0.2 }}
            >
              <p className='mb-3 flex items-center gap-2 text-base text-zinc-500'>
                Each <ActorsLogoWithIcon hoveredFeature={hoveredFeature} /> <span className='text-white'>Rivet Actor</span> has built-in:
              </p>
              <div className='flex flex-wrap gap-2'>
                {features.map((feature) => (
                  <button
                    key={feature}
                    type="button"
                    onClick={() => setHoveredFeature(hoveredFeature === feature ? null : feature)}
                    onMouseEnter={() => setHoveredFeature(feature)}
                    onMouseLeave={() => setHoveredFeature(null)}
                    className={`cursor-pointer rounded-full border px-3 py-1 text-xs transition-all bg-black/40 backdrop-blur-md ${
                      hoveredFeature === feature || highlightedFeatures.includes(feature)
                        ? 'border-white/30 text-white'
                        : hoveredFeature !== null || hoveredUseCase !== null
                          ? 'border-white/5 text-zinc-600'
                          : 'border-white/10 text-zinc-400'
                    }`}
                  >
                    {feature}
                  </button>
                ))}
              </div>
            </motion.div>

            <motion.div
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ duration: 0.5, delay: 0.3 }}
            >
              <p className='mb-3 text-base text-zinc-500'>
                And could be a:
              </p>
              <div className='flex flex-wrap gap-2'>
                {useCases.map((useCase) => (
                  <button
                    key={useCase}
                    type="button"
                    onClick={() => setHoveredUseCase(hoveredUseCase === useCase ? null : useCase)}
                    onMouseEnter={() => setHoveredUseCase(useCase)}
                    onMouseLeave={() => setHoveredUseCase(null)}
                    className={`cursor-pointer rounded-full border px-3 py-1 text-xs transition-all bg-black/40 backdrop-blur-md ${
                      activeUseCase === useCase || highlightedUseCases.includes(useCase)
                        ? 'border-white/30 text-white'
                        : hoveredFeature !== null || activeUseCase !== null
                          ? 'border-white/5 text-zinc-600'
                          : 'border-white/10 text-zinc-400'
                    }`}
                  >
                    {useCase}
                  </button>
                ))}
              </div>
            </motion.div>
          </div>
        </div>
      </div>

      {/* Bottom section - Each Actor (desktop only) */}
      <div
        className='left-0 right-0 px-6 pb-12 mt-auto hidden lg:block'
      >
        <div className='mx-auto w-full max-w-7xl'>
          {/* Changelog link - fades with scroll */}
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: scrollOpacity, filter: `blur(${(1 - scrollOpacity) * 8}px)` }}
            transition={{ duration: 0.5, delay: 0.15 }}
            className='mb-6'
          >
            <a
              href='/changelog'
              className='inline-flex items-center gap-2 rounded-full border border-white/10 px-2 py-0.5 text-xs text-zinc-400 transition-colors hover:border-white/20 hover:text-zinc-300'
            >
              <span className='h-[4px] w-[4px] bg-[#ff6030]' style={{ boxShadow: '0 0 2px #ffaa60, 0 0 4px #ff8040, 0 0 10px #ff6020, 0 0 20px rgba(255, 69, 0, 0.8)' }} />
              {latestChangelogTitle}
              <ArrowRight className='h-3 w-3' />
            </a>
          </motion.div>

          {/* Divider - fades with scroll */}
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: scrollOpacity, filter: `blur(${(1 - scrollOpacity) * 8}px)` }}
            transition={{ duration: 0.5, delay: 0.2 }}
            className='mb-8 h-px w-full bg-white/10'
          />

          <div className='grid gap-6 md:grid-cols-2'>
            <motion.div
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ duration: 0.5, delay: 0.2 }}
            >
              <p className='mb-3 flex items-center gap-2 text-base text-zinc-500'>
                Each <ActorsLogoWithIcon hoveredFeature={hoveredFeature} /> <span className='text-white'>Rivet Actor</span> has built-in:
              </p>
              <div className='flex flex-wrap gap-2'>
                {features.map((feature) => (
                  <span
                    key={feature}
                    onMouseEnter={() => setHoveredFeature(feature)}
                    onMouseLeave={() => setHoveredFeature(null)}
                    className={`cursor-default rounded-full border px-3 py-1 text-xs transition-all bg-black/40 backdrop-blur-md ${
                      hoveredFeature === feature || highlightedFeatures.includes(feature)
                        ? 'border-white/30 text-white'
                        : hoveredFeature !== null || hoveredUseCase !== null
                          ? 'border-white/5 text-zinc-600'
                          : 'border-white/10 text-zinc-400'
                    }`}
                  >
                    {feature}
                  </span>
                ))}
              </div>
            </motion.div>

            <motion.div
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ duration: 0.5, delay: 0.3 }}
            >
              <p className='mb-3 text-base text-zinc-500'>
                And could be a:
              </p>
              <div className='flex flex-wrap gap-2'>
                {useCases.map((useCase) => (
                  <span
                    key={useCase}
                    onMouseEnter={() => setHoveredUseCase(useCase)}
                    onMouseLeave={() => setHoveredUseCase(null)}
                    className={`cursor-default rounded-full border px-3 py-1 text-xs transition-all bg-black/40 backdrop-blur-md ${
                      activeUseCase === useCase || highlightedUseCases.includes(useCase)
                        ? 'border-white/30 text-white'
                        : hoveredFeature !== null || activeUseCase !== null
                          ? 'border-white/5 text-zinc-600'
                          : 'border-white/10 text-zinc-400'
                    }`}
                  >
                    {useCase}
                  </span>
                ))}
              </div>
            </motion.div>
          </div>
        </div>
      </div>
    </section>
  );
};
