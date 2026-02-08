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
      <span className='w-40 text-[#FF4500]'>session state</span>
      <span className='flex-1 border-t border-dashed border-[#FF4500]/30' />
      <span className='text-[#FF4500]'>Actor <span className='text-[#FF4500]/60'>(State)</span></span>
    </div>
    <div className='flex items-center gap-4'>
      <span className='w-40 text-[#FF4500]'>persistence</span>
      <span className='flex-1 border-t border-dashed border-[#FF4500]/30' />
      <span className='text-[#FF4500]'>Actor <span className='text-[#FF4500]/60'>(Storage)</span></span>
    </div>
    <div className='flex items-center gap-4'>
      <span className='w-40 text-[#FF4500]'>workflows</span>
      <span className='flex-1 border-t border-dashed border-[#FF4500]/30' />
      <span className='text-[#FF4500]'>Actor <span className='text-[#FF4500]/60'>(Workflows)</span></span>
    </div>
    <div className='flex items-center gap-4'>
      <span className='w-40 text-[#FF4500]'>job queues</span>
      <span className='flex-1 border-t border-dashed border-[#FF4500]/30' />
      <span className='text-[#FF4500]'>Actor <span className='text-[#FF4500]/60'>(Scheduling)</span></span>
    </div>
    <div className='flex items-center gap-4'>
      <span className='w-40 text-[#FF4500]'>real-time streaming</span>
      <span className='flex-1 border-t border-dashed border-[#FF4500]/30' />
      <span className='text-[#FF4500]'>Actor <span className='text-[#FF4500]/60'>(WebSockets)</span></span>
    </div>
    <div className='flex items-center gap-4'>
      <span className='w-40 text-[#FF4500]'>background compute</span>
      <span className='flex-1 border-t border-dashed border-[#FF4500]/30' />
      <span className='text-[#FF4500]'>Actor <span className='text-[#FF4500]/60'>(Long-running)</span></span>
    </div>
    <div className='flex items-center gap-4'>
      <span className='w-40 text-[#FF4500]'>sleep, wake, retries</span>
      <span className='flex-1 border-t border-dashed border-[#FF4500]/30' />
      <span className='text-[#FF4500]'>Actor <span className='text-[#FF4500]/60'>(Lifecycle)</span></span>
    </div>
    <p className='mt-4 text-center text-white'>
      One lightweight primitive. All the capabilities.
    </p>
  </div>
);

export const ProblemSection = () => (
  <section id='problem' className='border-y border-white/5 bg-zinc-900/10 py-32'>
    <div className='mx-auto max-w-7xl px-6'>
      <div className='flex flex-col gap-16'>
        {/* Header */}
        <div className='max-w-3xl'>
          <motion.h2
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            transition={{ duration: 0.5 }}
            className='mb-6 text-3xl font-medium tracking-tight text-white md:text-5xl'
          >
            AI broke the way we build backends.
          </motion.h2>
          <motion.p
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            transition={{ duration: 0.5, delay: 0.1 }}
            className='text-lg leading-relaxed text-zinc-400'
          >
            Long-running tasks, persistent sessions, real-time streaming — AI apps need all of it.
            So teams stitch together 7+ services just to ship one feature.
          </motion.p>
        </div>

        {/* Before / After cards */}
        <div className='grid grid-cols-1 gap-8 md:grid-cols-2'>
          <motion.div
            initial={{ opacity: 0, x: -20 }}
            whileInView={{ opacity: 1, x: 0 }}
            viewport={{ once: true }}
            transition={{ duration: 0.5 }}
            className='relative overflow-hidden rounded-2xl border border-white/5 bg-zinc-900/30 p-6 backdrop-blur-sm'
          >
            <div className='absolute left-0 right-0 top-0 z-10 h-[1px] bg-gradient-to-r from-transparent via-white/20 to-transparent' />
            <div className='mb-6'>
              <h4 className='font-semibold text-zinc-400'>The status quo</h4>
            </div>
            <ServiceStack />
          </motion.div>

          <motion.div
            initial={{ opacity: 0, x: 20 }}
            whileInView={{ opacity: 1, x: 0 }}
            viewport={{ once: true }}
            transition={{ duration: 0.5 }}
            className='relative overflow-hidden rounded-2xl border border-[#FF4500]/20 bg-zinc-900/30 p-6 shadow-[0_0_50px_-12px_rgba(255,69,0,0.1)] backdrop-blur-md'
          >
            <div className='absolute left-0 right-0 top-0 z-10 h-[1px] bg-gradient-to-r from-transparent via-[#FF4500]/40 to-transparent' />
            <div className='pointer-events-none absolute inset-0 bg-gradient-to-b from-[#FF4500]/5 to-transparent' />
            <div className='mb-6'>
              <h4 className='font-semibold text-white'>One Actor</h4>
            </div>
            <ActorSolution />
          </motion.div>
        </div>
      </div>
    </div>
  </section>
);
