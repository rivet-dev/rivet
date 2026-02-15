'use client';

import { motion } from 'framer-motion';

const ActorLogo = () => (
  <svg width="16" height="16" viewBox="0 0 176 173" className="inline-block align-middle mr-1.5">
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

const ServiceStack = () => (
  <div className='flex flex-col gap-2 font-mono text-sm'>
    <div className='flex items-center gap-4'>
      <span className='w-40 text-zinc-500'>session state</span>
      <span className='flex-1 border-t border-dashed border-zinc-700' />
      <span className='text-zinc-400'>Redis</span>
    </div>
    <div className='flex items-center gap-4'>
      <span className='w-40 text-zinc-500'>persistence</span>
      <span className='flex-1 border-t border-dashed border-zinc-700' />
      <span className='text-zinc-400'>Postgres</span>
    </div>
    <div className='flex items-center gap-4'>
      <span className='w-40 text-zinc-500'>job queues</span>
      <span className='flex-1 border-t border-dashed border-zinc-700' />
      <span className='text-zinc-400'>Kafka</span>
    </div>
    <div className='flex items-center gap-4'>
      <span className='w-40 text-zinc-500'>workflows</span>
      <span className='flex-1 border-t border-dashed border-zinc-700' />
      <span className='text-zinc-400'>Temporal</span>
    </div>
    <div className='flex items-center gap-4'>
      <span className='w-40 text-zinc-500'>real-time streaming</span>
      <span className='flex-1 border-t border-dashed border-zinc-700' />
      <span className='text-zinc-400'>Socket.io</span>
    </div>
    <div className='flex items-center gap-4'>
      <span className='w-40 text-zinc-500'>background compute</span>
      <span className='flex-1 border-t border-dashed border-zinc-700' />
      <span className='text-zinc-400'>Long-running workers</span>
    </div>
    <div className='flex items-center gap-4'>
      <span className='w-40 text-zinc-500'>sleep, wake, retries</span>
      <span className='flex-1 border-t border-dashed border-zinc-700' />
      <span className='text-zinc-400'>Custom logic</span>
    </div>
    <p className='mt-4 text-center text-zinc-500'>
      7 services. Weeks of integration.
    </p>
  </div>
);

const ActorSolution = () => (
  <div className='flex flex-col gap-2 font-mono text-sm'>
    <div className='flex items-center gap-4'>
      <span className='w-40 text-white'>session state</span>
      <span className='flex-1 border-t border-dashed border-white/30' />
      <span className='text-white'>Actor <span className='text-[#FF4500]'>(State)</span></span>
    </div>
    <div className='flex items-center gap-4'>
      <span className='w-40 text-white'>persistence</span>
      <span className='flex-1 border-t border-dashed border-white/30' />
      <span className='text-white'>Actor <span className='text-[#FF4500]'>(Storage)</span></span>
    </div>
    <div className='flex items-center gap-4'>
      <span className='w-40 text-white'>workflows</span>
      <span className='flex-1 border-t border-dashed border-white/30' />
      <span className='text-white'>Actor <span className='text-[#FF4500]'>(Workflows)</span></span>
    </div>
    <div className='flex items-center gap-4'>
      <span className='w-40 text-white'>job queues</span>
      <span className='flex-1 border-t border-dashed border-white/30' />
      <span className='text-white'>Actor <span className='text-[#FF4500]'>(Scheduling)</span></span>
    </div>
    <div className='flex items-center gap-4'>
      <span className='w-40 text-white'>real-time streaming</span>
      <span className='flex-1 border-t border-dashed border-white/30' />
      <span className='text-white'>Actor <span className='text-[#FF4500]'>(WebSockets)</span></span>
    </div>
    <div className='flex items-center gap-4'>
      <span className='w-40 text-white'>background compute</span>
      <span className='flex-1 border-t border-dashed border-white/30' />
      <span className='text-white'>Actor <span className='text-[#FF4500]'>(Long-running)</span></span>
    </div>
    <div className='flex items-center gap-4'>
      <span className='w-40 text-white'>sleep, wake, retries</span>
      <span className='flex-1 border-t border-dashed border-white/30' />
      <span className='text-white'>Actor <span className='text-[#FF4500]'>(Lifecycle)</span></span>
    </div>
    <p className='mt-4 text-center text-white'>
      One lightweight primitive. All the capabilities built-in.
    </p>
  </div>
);

export const ProblemSection = () => (
  <section id='problem' className='border-y border-white/5 py-48'>
    <div className='mx-auto max-w-7xl px-6'>
      <div className='flex flex-col gap-12'>
        {/* Header */}
        <div className='grid grid-cols-1 gap-8 md:grid-cols-2 md:gap-16'>
          <div>
            <motion.h2
              initial={{ opacity: 0, y: 20 }}
              whileInView={{ opacity: 1, y: 0 }}
              viewport={{ once: true }}
              transition={{ duration: 0.5 }}
              className='mb-3 text-2xl font-normal tracking-tight text-white md:text-4xl'
            >
              AI broke the way we build backends.
            </motion.h2>
            <motion.p
              initial={{ opacity: 0, y: 20 }}
              whileInView={{ opacity: 1, y: 0 }}
              viewport={{ once: true }}
              transition={{ duration: 0.5, delay: 0.1 }}
              className='text-base leading-relaxed text-zinc-500'
            >
              Long-running tasks, persistent sessions, real-time streaming — AI apps need all of it.
              So teams stitch together 7+ services just to ship one feature.
            </motion.p>
          </div>
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            transition={{ duration: 0.5, delay: 0.15 }}
            className='flex items-end'
          >
            <p className='text-base leading-relaxed text-zinc-500'>
              An Actor is just a function. Import it like a library, write your logic, and these capabilities come built in — making Actors natively suited for agent memory, background jobs, game lobbies, and more.
            </p>
          </motion.div>
        </div>

        {/* Before / After cards */}
        <div className='grid grid-cols-1 gap-8 md:grid-cols-2'>
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            transition={{ duration: 0.5 }}
            className='border-t border-white/10 pt-6'
          >
            <div className='mb-6'>
              <h4 className='text-sm font-medium uppercase tracking-wider text-zinc-500'>The status quo</h4>
            </div>
            <ServiceStack />
          </motion.div>

          <motion.div
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            transition={{ duration: 0.5, delay: 0.1 }}
            className='border-t border-white/10 pt-6'
          >
            <div className='mb-6'>
              <h4 className='text-sm font-medium uppercase tracking-wider text-white'>One <ActorLogo />Actor</h4>
            </div>
            <ActorSolution />
          </motion.div>
        </div>
      </div>
    </div>
  </section>
);
