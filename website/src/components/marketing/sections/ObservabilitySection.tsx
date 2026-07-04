'use client';

import { Database, GitBranch, Activity, Terminal } from 'lucide-react';
import { motion } from 'framer-motion';
import { SECTION_H2_CLASS, SUBTITLE_CLASS } from '../typography';

const inspectorSrc =
  'https://assets.rivet.dev/repo/website/src/components/marketing/images/screenshots/rivet-actor-inspector.png';
const inspectorAspect = 2688 / 2018;

export const ObservabilitySection = () => {
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
                  <img
                    src={inspectorSrc}
                    alt='Rivet Actor Inspector'
                    className='absolute inset-0 h-full w-full object-cover'
                  />
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
