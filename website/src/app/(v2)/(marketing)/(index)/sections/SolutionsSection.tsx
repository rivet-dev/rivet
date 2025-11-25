'use client';

import {
  Bot,
  Users,
  MousePointer2,
  TrendingUp,
  CalendarClock,
  Globe,
  Radio,
  Network,
  GitBranch,
  ArrowRight
} from 'lucide-react';
import { motion } from 'framer-motion';

export const SolutionsSection = () => {
  const solutions = [
    {
      title: 'AI Agent',
      description: 'Build AI assistants with persistent conversation history and tool calling',
      icon: <Bot className='h-5 w-5' />,
      exampleUrl: 'https://github.com/rivet-dev/rivet/blob/main/examples/ai-agent/src/backend/registry.ts'
    },
    {
      title: 'Chat Room',
      description: 'Real-time messaging with automatic state persistence and broadcasting',
      icon: <Users className='h-5 w-5' />,
      exampleUrl: 'https://github.com/rivet-dev/rivet/blob/main/examples/chat-room/src/backend/registry.ts'
    },
    {
      title: 'Collaborative Canvas',
      description: 'Real-time cursor tracking and collaborative text placement across users',
      icon: <MousePointer2 className='h-5 w-5' />,
      exampleUrl: 'https://github.com/rivet-dev/rivet/blob/main/examples/cursors/src/backend/registry.ts'
    },
    {
      title: 'Stream Processing',
      description: 'Track top values from streaming data with real-time updates',
      icon: <TrendingUp className='h-5 w-5' />,
      exampleUrl: 'https://github.com/rivet-dev/rivet/blob/main/examples/stream/src/backend/registry.ts'
    },
    {
      title: 'Background Jobs',
      description: 'Scheduled reminders and recurring tasks with automatic state persistence',
      icon: <CalendarClock className='h-5 w-5' />,
      exampleUrl: 'https://github.com/rivet-dev/rivet/blob/main/examples/quickstart-scheduling/src/backend/registry.ts'
    },
    {
      title: 'Multi-Region',
      description: 'Multi-region game rooms with player tracking and regional deployment',
      icon: <Globe className='h-5 w-5' />,
      exampleUrl: 'https://github.com/rivet-dev/rivet/blob/main/examples/quickstart-multi-region/src/backend/registry.ts'
    },
    {
      title: 'Realtime Events',
      description: 'Real-time cursor synchronization with connection state and broadcasting',
      icon: <Radio className='h-5 w-5' />,
      exampleUrl: 'https://github.com/rivet-dev/rivet/blob/main/examples/quickstart-realtime/src/backend/registry.ts'
    },
    {
      title: 'Native WebSockets',
      description: 'Raw WebSocket handling with low-level connection management',
      icon: <Network className='h-5 w-5' />,
      exampleUrl: 'https://github.com/rivet-dev/rivet/blob/main/examples/quickstart-native-websockets/src/backend/registry.ts'
    },
    {
      title: 'Cross-Actor Actions',
      description: 'Coordinate multiple actors for inventory management and checkout workflows',
      icon: <GitBranch className='h-5 w-5' />,
      exampleUrl: 'https://github.com/rivet-dev/rivet/blob/main/examples/quickstart-cross-actor-actions/src/backend/registry.ts'
    },
  ];

  return (
    <section id='solutions' className='hidden md:block relative border-t border-white/10 bg-black py-32'>
      <div className='mx-auto max-w-7xl px-6'>
        <div className='mb-20 text-center'>
          <motion.h2
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            transition={{ duration: 0.5 }}
            className='mb-6 text-3xl font-medium tracking-tight text-white md:text-5xl'
          >
            Build anything stateful.
          </motion.h2>
          <motion.p
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            transition={{ duration: 0.5, delay: 0.1 }}
            className='mx-auto max-w-2xl text-lg leading-relaxed text-zinc-400'
          >
            If it needs to remember something, it belongs in an Actor.
          </motion.p>
        </div>

        <div className='grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-3'>
          {solutions.map((solution, idx) => (
            <motion.a
              key={idx}
              href={solution.exampleUrl}
              target='_blank'
              rel='noopener noreferrer'
              initial={{ opacity: 0, y: 20 }}
              whileInView={{ opacity: 1, y: 0 }}
              viewport={{ once: true }}
              transition={{ duration: 0.5, delay: idx * 0.05 }}
              className='group relative flex flex-col justify-between overflow-hidden rounded-xl border border-white/10 bg-white/[0.02] p-6 backdrop-blur-sm transition-all duration-300 hover:border-white/20 hover:bg-white/[0.05] hover:shadow-[0_0_30px_-10px_rgba(255,255,255,0.1)]'
            >
              {/* Top Shine Highlight */}
              <div className='absolute left-0 right-0 top-0 z-10 h-[1px] bg-gradient-to-r from-transparent via-white/20 to-transparent opacity-50 transition-opacity group-hover:opacity-100' />
              <div className='relative z-10 mb-4 flex items-center justify-between'>
                <div className='flex items-center gap-3'>
                  <div className='text-white/80'>{solution.icon}</div>
                  <h3 className='font-medium tracking-tight text-white'>{solution.title}</h3>
                </div>
                <ArrowRight className='h-4 w-4 text-zinc-600 transition-colors group-hover:text-white' />
              </div>
              <p className='relative z-10 text-sm leading-relaxed text-zinc-400'>{solution.description}</p>
            </motion.a>
          ))}
        </div>
      </div>
    </section>
  );
};
