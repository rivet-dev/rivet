'use client';

import { useEffect, useState } from 'react';
import { Server, Box, Database, Cpu, Check } from 'lucide-react';
import { motion } from 'framer-motion';

const ArchitectureComparison = () => {
  const [activeStep, setActiveStep] = useState(0);

  useEffect(() => {
    const interval = setInterval(() => {
      setActiveStep(prev => (prev + 1) % 4);
    }, 2000);
    return () => clearInterval(interval);
  }, []);

  return (
    <div className='grid w-full grid-cols-1 gap-8 md:grid-cols-2'>
      {/* Traditional Serverless */}
      <motion.div
        initial={{ opacity: 0, x: -20 }}
        whileInView={{ opacity: 1, x: 0 }}
        viewport={{ once: true }}
        transition={{ duration: 0.5 }}
        className='relative overflow-hidden rounded-2xl border border-white/5 bg-zinc-900/30 p-6 backdrop-blur-sm transition-colors duration-500 hover:bg-zinc-900/50'
      >
        {/* Top Shine Highlight */}
        <div className='absolute left-0 right-0 top-0 z-10 h-[1px] bg-gradient-to-r from-transparent via-white/20 to-transparent' />
        <div className='relative z-10 mb-6 flex items-center gap-2'>
          <div className='rounded bg-red-500/10 p-2 text-red-400'>
            <Server className='h-4 w-4' />
          </div>
          <h4 className='font-semibold text-white'>Traditional Serverless</h4>
        </div>

        <div className='relative flex h-48 flex-col items-center justify-center gap-8'>
          <div className='flex w-full items-center justify-center gap-8'>
            <div
              className={`rounded border px-4 py-2 ${
                activeStep === 0 ? 'border-white bg-white text-black' : 'border-zinc-700 text-zinc-500'
              } font-mono text-xs transition-colors`}
            >
              Client
            </div>
            <div
              className={`h-[1px] w-12 ${
                activeStep === 0 ? 'bg-white' : 'bg-zinc-800'
              } relative transition-colors`}
            >
              <div
                className={`absolute -top-1 right-0 h-2 w-2 border-r border-t ${
                  activeStep === 0 ? 'border-white' : 'border-zinc-800'
                } rotate-45`}
              />
            </div>
            <div
              className={`rounded border px-4 py-2 ${
                activeStep === 1 || activeStep === 3
                  ? 'border-blue-500 bg-blue-500/10 text-blue-400'
                  : 'border-zinc-700 text-zinc-500'
              } font-mono text-xs transition-colors`}
            >
              Lambda
            </div>
          </div>

          <div className='flex flex-col items-center gap-2'>
            <div
              className={`h-8 w-[1px] ${
                activeStep === 1 || activeStep === 2 ? 'bg-blue-500' : 'bg-zinc-800'
              } relative transition-colors`}
            >
              <div
                className={`absolute -left-1 bottom-0 h-2 w-2 border-b border-r ${
                  activeStep === 1 || activeStep === 2 ? 'border-blue-500' : 'border-zinc-800'
                } rotate-45`}
              />
            </div>
            <div
              className={`rounded border px-4 py-2 ${
                activeStep === 2
                  ? 'border-[#FF4500] bg-[#FF4500]/10 text-[#FF4500]'
                  : 'border-zinc-700 text-zinc-500'
              } font-mono text-xs transition-colors`}
            >
              External DB
            </div>
          </div>
        </div>
        <p className='mt-4 text-center text-xs text-zinc-500'>
          State must be fetched from a remote DB for every request.
          <br />
          <span className='text-red-400'>High Latency • Connection Limits</span>
        </p>
      </motion.div>

      {/* Rivet Actor */}
      <motion.div
        initial={{ opacity: 0, x: 20 }}
        whileInView={{ opacity: 1, x: 0 }}
        viewport={{ once: true }}
        transition={{ duration: 0.5 }}
        className='relative overflow-hidden rounded-2xl border border-white/10 bg-zinc-900/30 p-6 shadow-[0_0_50px_-12px_rgba(16,185,129,0.1)] backdrop-blur-md transition-shadow duration-500 hover:shadow-[0_0_50px_-12px_rgba(16,185,129,0.2)]'
      >
        {/* Top Shine Highlight (Green tinted for the 'hero' card) */}
        <div className='absolute left-0 right-0 top-0 z-10 h-[1px] bg-gradient-to-r from-transparent via-emerald-500/40 to-transparent' />
        <div className='pointer-events-none absolute inset-0 bg-gradient-to-b from-emerald-500/5 to-transparent' />
        <div className='relative z-10 mb-6 flex items-center gap-2'>
          <div className='rounded bg-emerald-500/10 p-2 text-emerald-500'>
            <Box className='h-4 w-4' />
          </div>
          <h4 className='font-semibold text-white'>Rivet Actor Model</h4>
        </div>

        <div className='relative flex h-48 items-center justify-center'>
          <div className='flex w-full items-center justify-center gap-8'>
            <div
              className={`rounded border px-4 py-2 ${
                activeStep % 2 === 0 ? 'border-white bg-white text-black' : 'border-zinc-700 text-zinc-500'
              } font-mono text-xs transition-colors`}
            >
              Client
            </div>
            <div
              className={`h-[1px] w-16 ${
                activeStep % 2 === 0 ? 'bg-white' : 'bg-zinc-800'
              } relative transition-colors`}
            >
              <div
                className={`absolute -top-1 right-0 h-2 w-2 border-r border-t ${
                  activeStep % 2 === 0 ? 'border-white' : 'border-zinc-800'
                } rotate-45`}
              />
              <div
                className={`absolute -top-1 left-0 h-2 w-2 border-b border-l ${
                  activeStep % 2 === 0 ? 'border-white' : 'border-zinc-800'
                } rotate-45`}
              />
            </div>

            {/* The Actor */}
            <div className='relative'>
              <div
                className={`h-32 w-32 rounded-xl border ${
                  activeStep % 2 !== 0
                    ? 'border-emerald-500 bg-emerald-500/10'
                    : 'border-zinc-700 bg-zinc-900/50'
                } flex flex-col items-center justify-center gap-2 transition-all`}
              >
                <div className='font-mono text-xs font-bold text-white'>Actor</div>
                <div className='h-[1px] w-full bg-white/10' />
                <div className='flex items-center gap-2'>
                  <Cpu className='h-3 w-3 text-zinc-400' />
                  <span className='text-[10px] text-zinc-400'>Compute</span>
                </div>
                <div className='flex items-center gap-2'>
                  <Database className='h-3 w-3 text-emerald-500' />
                  <span className='text-[10px] text-emerald-500'>In-Mem State</span>
                </div>
              </div>
              {/* Pulse effect */}
              {activeStep % 2 !== 0 && (
                <div className='absolute inset-0 animate-ping rounded-xl border border-emerald-500 opacity-20' />
              )}
            </div>
          </div>
        </div>
        <p className='mt-4 text-center text-xs text-zinc-500'>
          State lives <i>with</i> the compute in memory.
          <br />
          <span className='text-emerald-500'>Zero Latency • Realtime • Persistent</span>
        </p>
      </motion.div>
    </div>
  );
};

export const ConceptSection = () => (
  <section id='actors' className='border-y border-white/5 bg-zinc-900/10 py-32'>
    <div className='mx-auto max-w-7xl px-6'>
      <div className='flex flex-col gap-16'>
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          transition={{ duration: 0.5 }}
        >
          <h2 className='mb-6 text-3xl font-medium tracking-tight text-white md:text-5xl'>
            Think in Actors,
            <br />
            not just Functions.
          </h2>
        </motion.div>

        <div className='grid grid-cols-1 gap-8 md:grid-cols-2'>
          <motion.p
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            transition={{ duration: 0.5, delay: 0.1 }}
            className='text-lg leading-relaxed text-zinc-400'
          >
            <strong className='text-white'>What is an Actor?</strong>
            <br />
            An Actor is a tiny, isolated server that holds its own data in memory. Unlike a stateless function that
            forgets everything after it runs, an Actor remembers.
          </motion.p>
          <motion.p
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            transition={{ duration: 0.5, delay: 0.2 }}
            className='text-lg leading-relaxed text-zinc-400'
          >
            <strong className='text-white'>Why use them?</strong>
            <br />
            When you need to manage state — like a chat room, user session, or multiplayer document — fetching that data
            frequently from a database is slow and expensive. Actors combine state and compute, keeping your data in
            memory in the same place as your application code.
          </motion.p>
        </div>

        <div className='w-full'>
          <ArchitectureComparison />
        </div>
      </div>
    </div>
  </section>
);
