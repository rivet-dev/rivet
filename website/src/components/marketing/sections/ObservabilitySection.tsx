'use client';

import { Database, GitBranch, Activity, Terminal, Sun, Moon } from 'lucide-react';
import { motion } from 'framer-motion';
import { useState } from 'react';
import { SECTION_H2_CLASS, SUBTITLE_CLASS } from '../typography';

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
  // The light screenshot reads cleanly inside the light frame, so it is the
  // default; the toggle still offers dark.
  const [theme, setTheme] = useState<InspectorTheme>('light');

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
    <section className='border-t border-ink/10 py-16 md:py-32'>
      <div className='mx-auto max-w-7xl px-6'>
        <div className='relative'>
          {/* The light frame holding the inspector screenshot */}
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            transition={{ duration: 0.5 }}
            className='relative'
          >
            <div className='relative overflow-hidden rounded-xl border border-ink/10 bg-paper-mid'>
              <div className='relative p-3 md:p-4'>
                <div className='relative w-full overflow-hidden rounded-md' style={{ aspectRatio: inspectorAspect }}>
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
                <div className='absolute bottom-6 right-6 flex items-center gap-1 rounded-lg border border-white/15 bg-ink/70 p-1 backdrop-blur'>
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
                        className='relative flex h-7 w-7 items-center justify-center rounded-md'
                      >
                        {active && (
                          <motion.span
                            layoutId='inspector-theme-active'
                            className='absolute inset-0 rounded-md bg-white'
                            transition={{ type: 'spring', stiffness: 500, damping: 35 }}
                          />
                        )}
                        <Icon
                          className={`relative h-3.5 w-3.5 transition-colors ${
                            active ? 'text-ink' : 'text-white/50 hover:text-white'
                          }`}
                        />
                      </button>
                    );
                  })}
                </div>
              </div>
            </div>
          </motion.div>

          {/* Text content below */}
          <div className='mt-12'>
            <motion.div
              initial={{ opacity: 0, y: 20 }}
              whileInView={{ opacity: 1, y: 0 }}
              viewport={{ once: true }}
              transition={{ duration: 0.5 }}
            >
              <h2 className={`mb-2 ${SECTION_H2_CLASS}`}>Built-In Observability</h2>
            </motion.div>
            <motion.p
              initial={{ opacity: 0, y: 20 }}
              whileInView={{ opacity: 1, y: 0 }}
              viewport={{ once: true }}
              transition={{ duration: 0.5, delay: 0.1 }}
              className={`mb-6 max-w-xl ${SUBTITLE_CLASS}`}
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
                  className='border-t border-ink/10 py-6 pr-8'
                >
                  <div className='mb-2 text-olive'>{feat.icon}</div>
                  <h3 className='mb-1 text-sm font-medium text-ink'>{feat.title}</h3>
                  <p className='text-sm leading-relaxed text-ink-soft'>{feat.description}</p>
                </motion.div>
              ))}
            </div>
          </div>
        </div>
      </div>
    </section>
  );
};
