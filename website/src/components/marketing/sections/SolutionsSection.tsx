'use client';

import { motion } from 'framer-motion';
import { ArrowRight, BrainCircuit, Gamepad2, Users, Database, Workflow, Plug } from 'lucide-react';

const solutions: Solution[] = [
  {
    icon: BrainCircuit,
    title: 'AI Agents',
    description: 'Long-running agents with persistent memory and context.',
    traditional: 'Shared database for all agent sessions, manual state management, timeouts on long tasks',
    withActors: 'Each agent is an actor with its own memory, streams updates in real-time, hibernates when idle',
    href: '/docs/actors',
  },
  {
    icon: Gamepad2,
    title: 'Game Servers',
    description: 'Real-time multiplayer with authoritative state.',
    traditional: 'Dedicated servers per region, complex matchmaking, scaling infrastructure',
    withActors: 'One actor per game room, built-in WebSockets, scales to millions of rooms',
    href: '/docs/actors/events',
  },
  {
    icon: Users,
    title: 'Collaboration',
    description: 'Shared state with real-time sync.',
    traditional: 'CRDT libraries, separate WebSocket servers, external pub/sub',
    withActors: 'Actor holds document state, broadcasts changes to all connected clients',
    href: '/docs/actors/state',
  },
  {
    icon: Database,
    title: 'Per-Tenant Data',
    description: 'Isolated databases per customer or user.',
    traditional: 'Row-level security, shared tables, complex access control',
    withActors: 'Each tenant gets their own actor with isolated SQLite database',
    href: '/docs/actors/kv',
  },
  {
    icon: Workflow,
    title: 'Workflows',
    description: 'Durable execution with retries and scheduling.',
    traditional: 'External workflow engine, message queues, separate scheduler',
    withActors: 'Built-in workflow runtime, scheduling, and state, all in one actor',
    href: '/docs/actors/schedule',
  },
  {
    icon: Plug,
    title: 'Your Existing Stack',
    description: 'Add actors alongside what you already have.',
    traditional: 'Adopt a whole new platform, migrate your database, rewrite your backend',
    withActors: 'Actors supplement your existing infrastructure. Keep your database, add actors where you need state',
    href: '/docs/connect',
  },
];

export const SolutionsSection = () => (
  <section className='border-t border-white/5 py-48'>
    <div className='mx-auto max-w-7xl px-6'>
      <div className='mb-12'>
        <motion.h2
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          transition={{ duration: 0.5 }}
          className='mb-2 text-2xl font-normal tracking-tight text-white md:text-4xl'
        >
          A different architecture.
        </motion.h2>
        <motion.p
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          transition={{ duration: 0.5, delay: 0.1 }}
          className='max-w-2xl text-base leading-relaxed text-zinc-500'
        >
          A simple primitive for stateful software with serverless benefits. No services to stitch, no database to share, no infrastructure to scale. Lightweight enough to run one per agent, session, or user, and they hibernate when idle.
        </motion.p>
      </div>

      <motion.div
        initial={{ opacity: 0, y: 20 }}
        whileInView={{ opacity: 1, y: 0 }}
        viewport={{ once: true }}
        transition={{ duration: 0.5 }}
        className='grid grid-cols-1 gap-8 md:grid-cols-2 lg:grid-cols-3'
      >
        {solutions.map((solution) => {
          const Icon = solution.icon;
          return (
            <div key={solution.title} className='flex flex-col border-t border-white/10 pt-6'>
              <div className='mb-3 text-zinc-500'>
                <Icon className='h-4 w-4' />
              </div>
              <h3 className='mb-2 text-base font-normal text-white'>{solution.title}</h3>
              <p className='mb-4 text-sm leading-relaxed text-zinc-500'>{solution.description}</p>

              <div className='mb-4 space-y-3'>
                <div className='rounded-md bg-white/5 p-3'>
                  <p className='mb-1 text-xs font-medium uppercase tracking-wider text-zinc-600'>Traditional</p>
                  <p className='text-xs text-zinc-500'>{solution.traditional}</p>
                </div>
                <div className='rounded-md border border-[#FF4500]/20 bg-[#FF4500]/5 p-3'>
                  <p className='mb-1 text-xs font-medium uppercase tracking-wider text-[#FF4500]/80'>With Actors</p>
                  <p className='text-xs text-zinc-400'>{solution.withActors}</p>
                </div>
              </div>

              <div className='mt-auto'>
                <a
                  href={solution.href}
                  className='group inline-flex items-center gap-2 text-sm text-zinc-500 transition-colors hover:text-white'
                >
                  Learn more
                  <ArrowRight className='h-3 w-3 transition-transform group-hover:translate-x-1' />
                </a>
              </div>
            </div>
          );
        })}
      </motion.div>
    </div>
  </section>
);
