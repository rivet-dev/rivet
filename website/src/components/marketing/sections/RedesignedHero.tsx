'use client';

import { useState } from 'react';
import { Terminal, ArrowRight, Check, Database, HardDrive, GitBranch, Clock, Wifi, Infinity, Moon } from 'lucide-react';
import { motion, AnimatePresence } from 'framer-motion';

const ActorsLogoWithIcon = ({ hoveredFeature }: { hoveredFeature: string | null }) => {
  const iconMap: Record<string, typeof Database> = {
    'In-memory state': Database,
    'Persistent storage': HardDrive,
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
    <button
      onClick={handleCopy}
      className='font-v2 inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md border border-white/10 bg-white/5 px-4 py-2 text-sm text-white subpixel-antialiased shadow-sm transition-colors hover:border-white/20'
    >
      {copied ? <Check className='h-4 w-4' /> : <Terminal className='h-4 w-4' />}
      npx skills add rivet-dev/skills
    </button>
  );
};

interface RedesignedHeroProps {
  latestChangelogTitle: string;
}

export const RedesignedHero = ({ latestChangelogTitle }: RedesignedHeroProps) => {
  const [hoveredFeature, setHoveredFeature] = useState<string | null>(null);
  const features = ['In-memory state', 'Persistent storage', 'Workflows', 'Scheduling', 'WebSockets', 'Runs indefinitely', 'Sleeps when idle'];

  return (
    <section className='relative overflow-hidden pb-20 pt-32 md:pb-32 md:pt-48'>
      <div className='pointer-events-none absolute left-1/2 top-0 h-[500px] w-[1000px] -translate-x-1/2 rounded-full bg-white/[0.02] blur-[100px]' />

      <div className='relative z-10 mx-auto max-w-7xl px-6'>
        <div className='flex flex-col items-center'>
          <div className='max-w-3xl flex-1 text-center'>
            <motion.div
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ duration: 0.5 }}
              className='mb-6'
            >
              <a href='/changelog'
                className='inline-flex items-center gap-2 rounded-full border border-white/10 bg-white/5 px-3 py-1 text-xs font-medium text-zinc-400 transition-colors hover:border-white/20'
              >
                <span className='h-2 w-2 animate-pulse rounded-full bg-[#FF4500]' />
                {latestChangelogTitle}
                <ArrowRight className='ml-1 h-3 w-3' />
              </a>
            </motion.div>

            <motion.h1
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ duration: 0.5, delay: 0.1 }}
              className='mb-8 text-5xl font-medium leading-[1.1] tracking-tighter text-white md:text-7xl'
            >
              Infrastructure for <br />
              software that thinks.
            </motion.h1>

            <motion.div
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ duration: 0.5, delay: 0.2 }}
              className='mx-auto mb-10'
            >
              <p className='mb-4 flex items-center justify-center gap-2 text-lg text-zinc-400'>
                Each <ActorsLogoWithIcon hoveredFeature={hoveredFeature} /> <span className='text-white'>Rivet Actor</span> has built in:
              </p>
              <div className='flex flex-wrap justify-center gap-2'>
                {features.map((feature) => (
                  <span
                    key={feature}
                    onMouseEnter={() => setHoveredFeature(feature)}
                    onMouseLeave={() => setHoveredFeature(null)}
                    className={`cursor-default rounded-full border px-3 py-1 text-sm transition-all ${
                      hoveredFeature === feature
                        ? 'border-[#FF4500]/40 bg-[#FF4500]/20 text-[#FF4500]'
                        : 'border-[#FF4500]/20 bg-[#FF4500]/10 text-[#FF4500]'
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
              className='flex flex-col items-center justify-center gap-4 sm:flex-row'
            >
              <div className='group flex flex-col items-center'>
                <a href='/docs'
                  className='font-v2 inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md border border-white/10 bg-white px-4 py-2 text-sm text-black subpixel-antialiased shadow-sm transition-colors hover:bg-zinc-200'
                >
                  Start Building
                  <ArrowRight className='h-4 w-4' />
                </a>
                <span className='mt-2 h-4 font-mono text-xs text-zinc-500 opacity-0 transition-opacity group-hover:opacity-100'>
                  for humans
                </span>
              </div>
              <div className='group flex flex-col items-center'>
                <CopyInstallButton />
                <span className='mt-2 h-4 font-mono text-xs text-zinc-500 opacity-0 transition-opacity group-hover:opacity-100'>
                  for coding agents
                </span>
              </div>
            </motion.div>
          </div>
        </div>
      </div>
    </section>
  );
};
