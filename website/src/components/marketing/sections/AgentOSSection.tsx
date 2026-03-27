'use client';

import { ArrowRight, Shield, Terminal, FolderOpen, Clock, Layers, Globe } from 'lucide-react';
import { motion } from 'framer-motion';
import agentosLogo from '@/images/products/agentos-logo.svg';

const features = [
  {
    icon: Terminal,
    title: 'Your tools, ready to go',
    description: 'Git, curl, Python, npm. The tools agents already know.',
  },
  {
    icon: Clock,
    title: 'Instant coldstart',
    description: '~5ms startup. No waiting for VMs to boot.',
  },
  {
    icon: FolderOpen,
    title: 'Real file system',
    description: 'A real, persistent file system agents can navigate like any Linux environment.',
  },
  {
    icon: Shield,
    title: 'Granular security',
    description: 'V8 isolates + WebAssembly. Hardware-level isolation without the overhead.',
  },
  {
    icon: Layers,
    title: 'Hybrid execution',
    description: 'Lightweight isolation by default. Full sandboxes when you need them.',
  },
  {
    icon: Globe,
    title: 'Runs anywhere',
    description: 'Railway, Kubernetes, browsers, edge. Just npm install and go.',
  },
];

export const AgentOSSection = () => (
  <section className='border-t border-white/10 py-16 md:py-48'>
    <div className='mx-auto max-w-7xl px-6'>
      {/* Header */}
      <div className='mb-16 max-w-3xl'>
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          transition={{ duration: 0.5 }}
          className='mb-4 flex items-center gap-3'
        >
          <img
            src={agentosLogo.src}
            alt='AgentOS'
            className='h-8 w-8'
          />
          <span className='font-mono text-xs font-semibold uppercase tracking-widest text-zinc-500'>
            AgentOS
          </span>
        </motion.div>
        <motion.h2
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          transition={{ duration: 0.5, delay: 0.05 }}
          className='mb-4 text-2xl font-normal tracking-tight text-white md:text-4xl'
        >
          Need more than primitives?
        </motion.h2>
        <motion.p
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          transition={{ duration: 0.5, delay: 0.1 }}
          className='text-base leading-relaxed text-zinc-500 md:text-lg'
        >
          Actors give you building blocks. AgentOS gives agents a complete environment.
          A lightweight VM with a real file system, real tools, and security via V8 isolates.
          Built on Actors underneath. Think of Actors as Unix and AgentOS as Linux.
        </motion.p>
      </div>

      {/* Features grid */}
      <motion.div
        initial={{ opacity: 0, y: 20 }}
        whileInView={{ opacity: 1, y: 0 }}
        viewport={{ once: true }}
        transition={{ duration: 0.5, delay: 0.15 }}
        className='mb-12 grid grid-cols-1 gap-px overflow-hidden rounded-lg border border-white/10 bg-white/5 sm:grid-cols-2 lg:grid-cols-3'
      >
        {features.map((feature) => (
          <div
            key={feature.title}
            className='flex flex-col gap-3 bg-black p-6 transition-colors hover:bg-white/[0.02]'
          >
            <feature.icon className='h-5 w-5 text-zinc-500' />
            <h3 className='text-sm font-medium text-white'>{feature.title}</h3>
            <p className='text-sm leading-relaxed text-zinc-500'>{feature.description}</p>
          </div>
        ))}
      </motion.div>

      {/* CTA */}
      <motion.div
        initial={{ opacity: 0, y: 20 }}
        whileInView={{ opacity: 1, y: 0 }}
        viewport={{ once: true }}
        transition={{ duration: 0.5, delay: 0.2 }}
        className='flex flex-col gap-4 sm:flex-row sm:items-center'
      >
        <a
          href='/agent-os'
          className='inline-flex items-center gap-2 text-sm text-zinc-400 transition-colors hover:text-white'
        >
          Learn more about AgentOS
          <ArrowRight className='h-3.5 w-3.5' />
        </a>
        <code className='rounded border border-white/10 bg-white/5 px-3 py-1.5 font-mono text-xs text-zinc-500'>
          npm install @rivetkit/agent-os
        </code>
      </motion.div>
    </div>
  </section>
);
