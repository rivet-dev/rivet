'use client';

import { Github, Cloud, Server, Check } from 'lucide-react';
import { motion } from 'framer-motion';

export const HostingSection = () => (
  <section className='border-t border-white/10 bg-black py-32'>
    <div className='mx-auto max-w-7xl px-6'>
      <div className='mb-16 text-center'>
        <motion.h2
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          transition={{ duration: 0.5 }}
          className='mb-6 text-3xl font-medium tracking-tight text-white md:text-5xl'
        >
          Deploy your way.
        </motion.h2>
        <motion.p
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          transition={{ duration: 0.5, delay: 0.1 }}
          className='mx-auto max-w-2xl text-lg leading-relaxed text-zinc-400'
        >
          Start with the open-source binary on your laptop. Scale with Rivet Cloud. Go hybrid when you need
          total control over data residency.
        </motion.p>
      </div>

      <div className='grid grid-cols-1 gap-8 md:grid-cols-3'>
        {/* Card 1: Self-Host */}
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          transition={{ duration: 0.5, delay: 0.2 }}
          className='relative flex flex-col overflow-hidden rounded-2xl border border-white/10 bg-white/[0.02] p-8 backdrop-blur-sm transition-all duration-300 hover:border-white/20 hover:bg-white/[0.05] hover:shadow-[0_0_30px_-10px_rgba(255,255,255,0.1)]'
        >
          {/* Top Shine Highlight */}
          <div className='absolute left-0 right-0 top-0 z-10 h-[1px] bg-gradient-to-r from-transparent via-white/20 to-transparent' />
          <div className='relative z-10 mb-6 flex h-12 w-12 items-center justify-center rounded-lg bg-white/5 text-white'>
            <Server className='h-6 w-6' />
          </div>
          <h3 className='mb-3 text-xl font-medium text-white'>Self-Host</h3>
          <p className='mb-6 flex-grow text-sm leading-relaxed text-zinc-400'>
            Deploy as a single Rust binary or Docker container. Works with your existing Postgres, file system, or FoundationDB. Easy-to-use dashboard included.
          </p>
          <div className='mb-4 rounded-lg border border-white/10 bg-black p-4 font-mono text-[10px] text-zinc-300'>
            <div className='flex gap-2'>
              <span className='select-none text-[#FF4500]'>$</span>
              <span>docker run -p 6420:6420 rivetkit/engine</span>
            </div>
          </div>
          <a
            href='/docs/self-hosting'
            className='flex items-center justify-center gap-2 rounded-lg border border-white/20 bg-white/5 px-4 py-2 text-xs text-white transition-colors hover:bg-white/10'
          >
            View Self-Hosting Docs
          </a>
        </motion.div>

        {/* Card 2: Rivet Cloud */}
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          transition={{ duration: 0.5, delay: 0.3 }}
          className='group relative flex flex-col overflow-hidden rounded-2xl border border-white/10 bg-gradient-to-b from-[#FF4500]/10 to-transparent p-8 backdrop-blur-sm transition-colors hover:border-[#FF4500]/30'
        >
          {/* Top Shine Highlight (Orange) */}
          <div className='absolute left-0 right-0 top-0 z-10 h-[1px] bg-gradient-to-r from-transparent via-[#FF4500]/40 to-transparent' />
          <div className='relative z-10 mb-6 flex h-12 w-12 items-center justify-center rounded-lg bg-[#FF4500]/10 text-[#FF4500]'>
            <Cloud className='h-6 w-6' />
          </div>
          <h3 className='relative z-10 mb-3 text-xl font-medium text-white'>Rivet Cloud</h3>
          <p className='relative z-10 mb-6 flex-grow text-sm leading-relaxed text-zinc-400'>
            The fully managed actor platform. We handle the orchestration, monitoring, and edge routing. You
            just connect your compute and go.
          </p>
          <ul className='relative z-10 mb-4 space-y-2'>
            {['Global Edge Network', 'Scales Seamlessly', 'Connects To Your Cloud'].map(item => (
              <li key={item} className='flex items-center gap-2 text-xs text-zinc-300'>
                <Check className='h-3 w-3 text-[#FF4500]' /> {item}
              </li>
            ))}
          </ul>
          <a
            href='https://dashboard.rivet.dev'
            target='_blank'
            rel='noopener noreferrer'
            className='relative z-10 flex items-center justify-center gap-2 rounded-lg border border-white/10 bg-white px-4 py-2 text-xs text-black transition-colors hover:bg-zinc-200'
          >
            Sign Up
          </a>
        </motion.div>

        {/* Card 3: Open Source */}
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          transition={{ duration: 0.5, delay: 0.4 }}
          className='relative flex flex-col overflow-hidden rounded-2xl border border-white/10 bg-white/[0.02] p-8 backdrop-blur-sm transition-all duration-300 hover:border-white/20 hover:bg-white/[0.05] hover:shadow-[0_0_30px_-10px_rgba(255,255,255,0.1)]'
        >
          {/* Top Shine Highlight */}
          <div className='absolute left-0 right-0 top-0 z-10 h-[1px] bg-gradient-to-r from-transparent via-white/20 to-transparent' />
          <div className='relative z-10 mb-6 flex h-12 w-12 items-center justify-center rounded-lg bg-white/5 text-white'>
            <Github className='h-6 w-6' />
          </div>
          <h3 className='mb-3 text-xl font-medium text-white'>Open Source</h3>
          <p className='mb-6 flex-grow text-sm leading-relaxed text-zinc-400'>
            Built in Rust for performance and safety. Fully open source under Apache 2.0. Audit the code, contribute features, and hack it as you'd like.
          </p>
          <div className='mb-4 rounded-lg border border-white/10 bg-black p-4 font-mono text-[10px] text-zinc-300'>
            <div className='flex gap-2'>
              <span className='select-none text-[#FF4500]'>$</span>
              <span>git clone https://github.com/rivet-dev/rivet</span>
            </div>
            <div className='mt-1 flex gap-2'>
              <span className='select-none text-[#FF4500]'>$</span>
              <span>cd rivet</span>
            </div>
            <div className='mt-1 flex gap-2'>
              <span className='select-none text-[#FF4500]'>$</span>
              <span>cargo run -p rivet-engine</span>
            </div>
          </div>
          <a
            href='https://github.com/rivet-dev/rivet'
            target='_blank'
            rel='noopener noreferrer'
            className='flex items-center justify-center gap-2 rounded-lg border border-white/20 bg-white/5 px-4 py-2 text-xs text-white transition-colors hover:bg-white/10'
          >
            <Github className='h-3 w-3' />
            View on GitHub
          </a>
        </motion.div>
      </div>
    </div>
  </section>
);
