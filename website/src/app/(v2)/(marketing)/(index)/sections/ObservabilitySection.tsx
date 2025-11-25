'use client';

import { Eye, Activity, Terminal, Wifi, ArrowRight, Play } from 'lucide-react';
import { motion } from 'framer-motion';
import Image from 'next/image';
import imgInspector from '../images/screenshots/inspector-6.png';

export const ObservabilitySection = () => {
  const features = [
    {
      title: 'Live State Inspection',
      description: 'View and edit your actor state in real-time as messages are sent and processed',
      icon: <Eye className='h-5 w-5 text-emerald-400' />
    },
    {
      title: 'Network Inspector',
      description: 'Monitor active connections with state and parameters for each client',
      icon: <Wifi className='h-5 w-5 text-purple-400' />
    },
    {
      title: 'Event Monitoring',
      description:
        'See all events happening in your actor in real-time and track every state change and action as it happens',
      icon: <Activity className='h-5 w-5 text-blue-400' />
    },
    {
      title: 'REPL',
      description:
        'Debug your actor in real-time by calling actions, subscribing to events, and interacting directly with your code',
      icon: <Terminal className='h-5 w-5 text-[#FF4500]' />
    },
  ];

  return (
    <section className='border-t border-white/10 bg-black py-32'>
      <div className='mx-auto max-w-7xl px-6'>
        <div className='relative'>
          {/* Screenshot on top */}
          <motion.div
            initial={{ opacity: 0, scale: 0.95 }}
            whileInView={{ opacity: 1, scale: 1 }}
            viewport={{ once: true }}
            transition={{ duration: 0.7 }}
            className='relative'
          >
            <div className='absolute -inset-4 rounded-3xl bg-gradient-to-r from-[#FF4500]/20 to-blue-500/20 opacity-20 blur-2xl' />
            <div
              className='relative flex flex-col overflow-hidden rounded-xl border border-white/10 bg-zinc-900/50 shadow-2xl backdrop-blur-xl'
              style={{
                maskImage: 'radial-gradient(ellipse 300% 100% at 50% 90%, transparent 0%, black 25%)',
                WebkitMaskImage: 'radial-gradient(ellipse 300% 100% at 50% 90%, transparent 0%, black 25%)'
              }}
            >
              {/* Top Shine Highlight */}
              <div className='absolute left-0 right-0 top-0 z-10 h-[1px] bg-gradient-to-r from-transparent via-white/20 to-transparent' />
              {/* Window Bar */}
              <div className='flex items-center gap-2 border-b border-white/5 bg-white/5 px-4 py-3'>
                <div className='flex gap-1.5'>
                  <div className='h-3 w-3 rounded-full border border-zinc-500/50 bg-zinc-500/20' />
                  <div className='h-3 w-3 rounded-full border border-zinc-500/50 bg-zinc-500/20' />
                  <div className='h-3 w-3 rounded-full border border-zinc-500/50 bg-zinc-500/20' />
                </div>
              </div>
              {/* Content Area - Dashboard Image */}
              <div className='relative w-full bg-zinc-900/50' style={{ aspectRatio: `${imgInspector.width} / ${imgInspector.height}` }}>
                <Image
                  src={imgInspector}
                  alt='Rivet Inspector Dashboard'
                  className='h-full w-full object-cover'
                  fill
                  sizes='(max-width: 1024px) 100vw, 80vw'
                  priority
                  quality={90}
                />
              </div>
            </div>
          </motion.div>

          {/* Text content overlapping bottom */}
          <div className='relative -mt-32 px-6'>
            <div className='mx-auto max-w-4xl'>
              <motion.h2
                initial={{ opacity: 0, y: 20 }}
                whileInView={{ opacity: 1, y: 0 }}
                viewport={{ once: true }}
                transition={{ duration: 0.5 }}
                className='mb-6 text-3xl font-medium tracking-tight text-white md:text-5xl'
              >
                Built-In Observability
              </motion.h2>
              <motion.p
                initial={{ opacity: 0, y: 20 }}
                whileInView={{ opacity: 1, y: 0 }}
                viewport={{ once: true }}
                transition={{ duration: 0.5, delay: 0.1 }}
                className='mb-8 max-w-2xl text-lg leading-relaxed text-zinc-400'
              >
                Powerful debugging and monitoring tools that work seamlessly from local development to production
                at scale.
              </motion.p>
              <motion.div
                initial={{ opacity: 0, y: 20 }}
                whileInView={{ opacity: 1, y: 0 }}
                viewport={{ once: true }}
                transition={{ duration: 0.5, delay: 0.2 }}
                className='mb-12 flex flex-col gap-4 sm:flex-row'
              >
                <a
                  href='https://inspect.rivet.dev'
                  target='_blank'
                  rel='noopener noreferrer'
                  className='font-v2 inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md border border-white/10 bg-white/5 px-4 py-2 text-sm text-white subpixel-antialiased shadow-sm transition-colors hover:border-white/20'
                >
                  Visit The Inspector
                  <ArrowRight className='h-4 w-4' />
                </a>
                <a
                  href='https://x.com/NathanFlurry/status/1976427064678023634'
                  target='_blank'
                  rel='noopener noreferrer'
                  className='font-v2 inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md border border-white/10 bg-white/5 px-4 py-2 text-sm text-white subpixel-antialiased shadow-sm transition-colors hover:border-white/20'
                >
                  <Play className='h-4 w-4' />
                  Watch Demo
                </a>
              </motion.div>

              {/* Feature List */}
              <div className='grid gap-6 sm:grid-cols-2 lg:gap-8'>
                {features.map((feat, idx) => (
                  <motion.div
                    key={idx}
                    initial={{ opacity: 0, x: -20 }}
                    whileInView={{ opacity: 1, x: 0 }}
                    viewport={{ once: true }}
                    transition={{ duration: 0.5, delay: idx * 0.1 }}
                    className='flex gap-4'
                  >
                    <div className='flex h-10 w-10 flex-shrink-0 items-center justify-center rounded-lg border border-white/10 bg-zinc-900'>
                      {feat.icon}
                    </div>
                    <div>
                      <h3 className='mb-1 text-lg font-medium text-white'>{feat.title}</h3>
                      <p className='text-sm leading-relaxed text-zinc-400'>{feat.description}</p>
                    </div>
                  </motion.div>
                ))}
              </div>
            </div>
          </div>
        </div>
      </div>
    </section>
  );
};
