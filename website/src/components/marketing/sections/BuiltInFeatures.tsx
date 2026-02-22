'use client';

import { Database, HardDrive, GitBranch, Clock, Wifi, ListOrdered, Infinity, Layers, ArrowRight } from 'lucide-react';
import { motion } from 'framer-motion';

const RivetActorIcon = ({ className }: { className?: string }) => (
  <svg width="32" height="32" viewBox="0 0 176 173" className={className}>
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
    <g transform="translate(-32928.8,-28118.2)">
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
    </g>
  </svg>
);

const features = [
  {
    icon: Database,
    title: 'In-memory state',
    description: 'Co-located with compute for instant reads and writes.',
    href: '/docs/actors/state',
  },
  {
    icon: HardDrive,
    title: 'SQLite or BYO database',
    description: 'Persistent storage that survives restarts and deploys.',
    href: '/docs/actors/persistence',
  },
  {
    icon: Infinity,
    title: 'Runs indefinitely, sleeps when idle',
    description: 'Long-lived when active, hibernates when idle.',
    href: '/docs/actors/lifecycle',
  },
  {
    icon: Layers,
    title: 'Scales infinitely, scales to zero',
    description: 'Supports bursty workloads and is cost-efficient.',
    href: '/docs/actors/design-patterns',
  },
  {
    icon: Wifi,
    title: 'WebSockets',
    description: 'Real-time bidirectional streaming built in.',
    href: '/docs/actors/events',
  },
  {
    icon: GitBranch,
    title: 'Workflows',
    description: 'Multi-step operations with automatic retries.',
    href: '/docs/actors/workflows',
  },
  {
    icon: ListOrdered,
    title: 'Queues',
    description: 'Durable message queues for reliable async processing.',
    href: '/docs/actors/queues',
  },
  {
    icon: Clock,
    title: 'Scheduling',
    description: 'Timers and cron jobs within your actor.',
    href: '/docs/actors/schedule',
  },
];

export const BuiltInFeatures = () => {
  return (
    <section className='relative border-t border-b border-white/10 bg-white/[0.03] px-6 py-16 lg:py-24'>
      <div className='mx-auto w-full max-w-7xl'>
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          transition={{ duration: 0.5 }}
          className='mb-10'
        >
          <h2 className='mb-2 text-2xl font-normal tracking-tight text-zinc-400 md:text-4xl'>
            <span>The </span>
            <span className='ml-1 whitespace-nowrap text-white'>
              <RivetActorIcon className='mr-2 inline-block h-6 w-6 align-[-0.15em] md:h-8 md:w-8' />
              Rivet Actor
            </span>
            <span> is built for modern stateful workloads.</span>
          </h2>
          <p className='text-base leading-relaxed text-zinc-500'>
            One Actor per agent, per session, per user — each with everything it needs built in.
          </p>
        </motion.div>

        <div className='grid grid-cols-1 gap-x-8 gap-y-6 min-[440px]:grid-cols-2 sm:grid-cols-3 lg:grid-cols-4'>
          {features.map((feature, idx) => {
            const Icon = feature.icon;
            return (
              <motion.a
                key={feature.title}
                href={feature.href}
                initial={{ opacity: 0, y: 16 }}
                whileInView={{ opacity: 1, y: 0 }}
                viewport={{ once: true }}
                transition={{ duration: 0.4, delay: idx * 0.05 }}
                className='group flex items-start justify-between gap-3 rounded-xl px-4 py-4 -mx-4 border border-transparent transition-colors duration-200 hover:bg-[#FF4500]/[0.06] hover:border-[#FF4500]/[0.12]'
              >
                <div className='flex flex-col gap-2'>
                  <div className='flex items-center gap-2.5'>
                    <Icon className='h-4 w-4 text-zinc-500' />
                    <span className='text-sm font-medium text-white'>{feature.title}</span>
                  </div>
                  <p className='text-xs leading-relaxed text-zinc-500'>{feature.description}</p>
                </div>
                <ArrowRight className='h-4 w-4 mt-0.5 flex-shrink-0 text-[#FF4500] opacity-0 translate-x-0 transition-all duration-200 group-hover:opacity-100 group-hover:translate-x-1' />
              </motion.a>
            );
          })}
        </div>
      </div>
    </section>
  );
};
