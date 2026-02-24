'use client';

import { Eye, Activity, Terminal, Wifi, ArrowRight } from 'lucide-react';
import { motion } from 'framer-motion';
import imgInspector from '../images/screenshots/inspector-6.png';

export const ObservabilitySection = () => {
  const features = [
    {
      title: 'Live State Inspection',
      description: 'View and edit your actor state in real-time as messages are sent and processed',
      icon: <Eye className='h-4 w-4' />
    },
    {
      title: 'Network Inspector',
      description: 'Monitor active connections with state and parameters for each client',
      icon: <Wifi className='h-4 w-4' />
    },
    {
      title: 'Event Monitoring',
      description:
        'See all events happening in your actor in real-time and track every state change and action as it happens',
      icon: <Activity className='h-4 w-4' />
    },
    {
      title: 'REPL',
      description:
        'Debug your actor in real-time by calling actions, subscribing to events, and interacting directly with your code',
      icon: <Terminal className='h-4 w-4' />
    },
  ];

  return (
    <section className='border-t border-white/10 bg-black py-48'>
      <div className='mx-auto max-w-7xl px-6'>
        <div className='relative'>
          {/* Screenshot on top */}
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            transition={{ duration: 0.5 }}
            className='relative'
          >
            <div className='overflow-hidden rounded-lg border border-white/10'>
              <div className='relative w-full' style={{ aspectRatio: `${imgInspector.width} / ${imgInspector.height}` }}>
                <img src={imgInspector.src}
                  alt='Rivet Inspector Dashboard'
                  className='h-full w-full object-cover'
                />
              </div>
            </div>
          </motion.div>

          {/* Text content below */}
          <div className='mt-12'>
            <motion.h2
              initial={{ opacity: 0, y: 20 }}
              whileInView={{ opacity: 1, y: 0 }}
              viewport={{ once: true }}
              transition={{ duration: 0.5 }}
              className='mb-2 text-2xl font-normal tracking-tight text-white md:text-4xl'
            >
              Built-In Observability
            </motion.h2>
            <motion.p
              initial={{ opacity: 0, y: 20 }}
              whileInView={{ opacity: 1, y: 0 }}
              viewport={{ once: true }}
              transition={{ duration: 0.5, delay: 0.1 }}
              className='mb-6 max-w-xl text-base leading-relaxed text-zinc-500'
            >
              Powerful debugging and monitoring tools that work seamlessly from local development to production
              at scale.
            </motion.p>
            {/* Feature List */}
            <div className='grid gap-px sm:grid-cols-2'>
              {features.map((feat, idx) => (
                <motion.div
                  key={idx}
                  initial={{ opacity: 0, y: 20 }}
                  whileInView={{ opacity: 1, y: 0 }}
                  viewport={{ once: true }}
                  transition={{ duration: 0.5, delay: idx * 0.1 }}
                  className='border-t border-white/10 py-6 pr-8'
                >
                  <div className='mb-2 text-zinc-600'>{feat.icon}</div>
                  <h3 className='mb-1 text-sm font-normal text-white'>{feat.title}</h3>
                  <p className='text-sm leading-relaxed text-zinc-500'>{feat.description}</p>
                </motion.div>
              ))}
            </div>
          </div>
        </div>
      </div>
    </section>
  );
};
