'use client';

import { motion } from 'framer-motion';

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
      One lightweight primitive. All the capabilities.
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
              <h4 className='text-sm font-medium uppercase tracking-wider text-white'>One Actor</h4>
            </div>
            <ActorSolution />
          </motion.div>
        </div>
      </div>
    </div>
  </section>
);
