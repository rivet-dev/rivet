'use client';

import { useState } from 'react';
import { ArrowRight, Shield, FolderOpen, Clock, Globe, Bot, Code, Terminal, Check } from 'lucide-react';
import { motion } from 'framer-motion';

const CopyInstallButton = () => {
  const [copied, setCopied] = useState(false);

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText('npm install @rivetkit/agent-os');
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch (err) {
      console.error('Failed to copy:', err);
    }
  };

  return (
    <button
      onClick={handleCopy}
      className='inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md border border-white/10 bg-white/5 px-4 py-2 text-sm text-zinc-400 transition-colors hover:border-white/20 hover:text-white'
    >
      {copied ? <Check className='h-4 w-4 text-green-500' /> : <Terminal className='h-4 w-4' />}
      npm install @rivetkit/agent-os
    </button>
  );
};

const features = [
  {
    icon: Bot,
    title: 'Works with any agent',
    description: 'Claude Code, Codex, OpenCode, and more. One unified API.',
  },
  {
    icon: Clock,
    title: 'Low overhead',
    description: '~5ms coldstart. 200x cheaper than sandboxes.',
  },
  {
    icon: Code,
    title: 'Embed in your backend',
    description: 'Your APIs. Your toolchains. No complex agent authentication.',
  },
  {
    icon: FolderOpen,
    title: 'Mount anything as a file system',
    description: 'S3, SQLite, Google Drive, or the host file system.',
  },
  {
    icon: Shield,
    title: 'Granular security',
    description: 'V8 isolates + WebAssembly. Configurable network and file system policies.',
  },
  {
    icon: Globe,
    title: 'Runs anywhere',
    description: 'Rivet Cloud, Railway, Vercel, Kubernetes, or on-prem.',
  },
];

export const AgentOSSection = () => (
  <section className='border-t border-white/10 py-16 md:py-48'>
    <div className='mx-auto max-w-7xl px-6'>
      {/* Header */}
      <div className='mb-16 max-w-3xl'>
        <motion.h2
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          transition={{ duration: 0.5, delay: 0.05 }}
          className='mb-4 text-2xl font-normal tracking-tight text-white md:text-4xl'
        >
          Need more than primitives? Try AgentOS.
        </motion.h2>
        <motion.p
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          transition={{ duration: 0.5, delay: 0.1 }}
          className='text-base leading-relaxed text-zinc-500 md:text-lg'
        >
          Unix gave humans a common language to control machines. AgentOS gives agents the same power.
          A lightweight runtime with a real file system, real tools, and security via V8 isolates.
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
        <CopyInstallButton />
      </motion.div>
    </div>
  </section>
);
