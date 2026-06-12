'use client';

import { Database, GitBranch, Activity, Terminal, ArrowRight, Sun, Moon } from 'lucide-react';
import { motion } from 'framer-motion';
import { useState } from 'react';

const inspectorAspect = 2438 / 1613;

const inspectorImages = {
  light: {
    src: 'https://assets.rivet.dev/repo/website/src/components/marketing/images/screenshots/rivet-actor-inspector-light.png',
    icon: Sun,
    label: 'Show inspector in light mode',
  },
  dark: {
    src: 'https://assets.rivet.dev/repo/website/src/components/marketing/images/screenshots/rivet-actor-inspector-dark.png',
    icon: Moon,
    label: 'Show inspector in dark mode',
  },
} as const;

type InspectorTheme = keyof typeof inspectorImages;
const inspectorThemes = Object.keys(inspectorImages) as InspectorTheme[];

export const ObservabilitySection = () => {
  const [theme, setTheme] = useState<InspectorTheme>('dark');

  const features = [
    {
      title: 'SQLite Viewer',
      description: 'Browse and query SQLite databases in real-time across actors and agent sessions',
      icon: <Database className='h-4 w-4' />
    },
    {
      title: 'Workflow State',
      description: 'Inspect workflow progress, steps, and retries as they execute',
      icon: <GitBranch className='h-4 w-4' />
    },
    {
      title: 'Event Monitoring',
      description: 'Track every state change, action, and agent event as it happens in real-time',
      icon: <Activity className='h-4 w-4' />
    },
    {
      title: 'REPL',
      description:
        'Debug actors and agent sessions by calling actions, subscribing to events, and interacting directly with your code',
      icon: <Terminal className='h-4 w-4' />
    },
  ];

  return (
    <section className='border-t border-white/10 bg-black py-16 md:py-48'>
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
            <div className='relative overflow-hidden rounded-lg border border-white/10'>
              <div className='relative w-full' style={{ aspectRatio: inspectorAspect }}>
                {inspectorThemes.map((t) => (
                  <motion.img
                    key={t}
                    src={inspectorImages[t].src}
                    alt={`Rivet Actor Inspector in ${t} mode`}
                    className='absolute inset-0 h-full w-full object-cover'
                    initial={false}
                    animate={{ opacity: theme === t ? 1 : 0 }}
                    transition={{ duration: 0.35, ease: 'easeInOut' }}
                  />
                ))}
              </div>

              {/* Light/dark mode toggle */}
              <div className='absolute bottom-3 right-3 flex items-center gap-1 rounded-full border border-white/10 bg-black/40 p-1 backdrop-blur'>
                {inspectorThemes.map((t) => {
                  const Icon = inspectorImages[t].icon;
                  const active = theme === t;
                  return (
                    <button
                      key={t}
                      type='button'
                      onClick={() => setTheme(t)}
                      aria-label={inspectorImages[t].label}
                      aria-pressed={active}
                      className='relative flex h-7 w-7 items-center justify-center rounded-full'
                    >
                      {active && (
                        <motion.span
                          layoutId='inspector-theme-active'
                          className='absolute inset-0 rounded-full bg-white'
                          transition={{ type: 'spring', stiffness: 500, damping: 35 }}
                        />
                      )}
                      <Icon
                        className={`relative h-3.5 w-3.5 transition-colors ${
                          active ? 'text-black' : 'text-zinc-400 hover:text-white'
                        }`}
                      />
                    </button>
                  );
                })}
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
              className='mb-2 text-2xl font-medium tracking-tight text-white md:text-4xl'
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
              Debugging and monitoring for actors and agents, from local development to production at scale.
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
                  <h3 className='mb-1 text-sm font-medium text-white'>{feat.title}</h3>
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
