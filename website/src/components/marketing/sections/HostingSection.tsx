'use client';

import { Package, Server, Check, ArrowRight, Github } from 'lucide-react';
import { motion } from 'framer-motion';
import imgLogo from '@/images/rivet-logos/icon-white.svg';

export const HostingSection = () => (
  <section className='border-t border-white/10 py-16 md:py-48'>
    <div className='mx-auto max-w-7xl px-6'>
      <div className='mb-12'>
        <motion.h2
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          transition={{ duration: 0.5 }}
          className='mb-2 text-2xl font-normal tracking-tight text-white md:text-4xl'
        >
          Start local. Scale to millions.
        </motion.h2>
        <motion.p
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          transition={{ duration: 0.5, delay: 0.1 }}
          className='max-w-xl text-base leading-relaxed text-zinc-500'
        >
          RivetKit is a library. Connect it to Rivet Cloud or self-host when you need scaling, fault tolerance, and observability.
        </motion.p>
      </div>

      <motion.div
        initial={{ opacity: 0, y: 20 }}
        whileInView={{ opacity: 1, y: 0 }}
        viewport={{ once: true }}
        transition={{ duration: 0.5 }}
        className='grid grid-cols-1 gap-8 md:grid-cols-3'
      >
        {/* Card 1: Just a Library */}
        <div className='flex flex-col border-t border-white/10 pt-6'>
          <div className='mb-3 text-zinc-500'>
            <Package className='h-4 w-4' />
          </div>
          <h3 className='mb-2 text-base font-normal text-white'>Just a Library</h3>
          <p className='mb-6 text-sm leading-relaxed text-zinc-500'>
            Install a package and run locally. No servers, no infrastructure. Actors run in your process during development.
          </p>
          <div className='mb-6 font-mono text-xs text-zinc-500'>
            <div className='flex gap-2'>
              <span className='select-none text-zinc-600'>$</span>
              <span>npm install rivetkit</span>
            </div>
          </div>
          <div className='mt-auto'>
            <a
              href='/docs/actors/quickstart'
              className='inline-flex items-center justify-center whitespace-nowrap rounded-md border border-white/10 px-4 py-2 text-sm text-zinc-300 transition-colors hover:border-white/20 hover:text-white'
            >
              Get Started
            </a>
          </div>
        </div>

        {/* Card 2: Self-Host */}
        <div className='flex flex-col border-t border-white/10 pt-6'>
          <div className='mb-3 text-zinc-500'>
            <Server className='h-4 w-4' />
          </div>
          <h3 className='mb-2 text-base font-normal text-white'>Self-Host</h3>
          <p className='mb-6 text-sm leading-relaxed text-zinc-500'>
            Single Rust binary or Docker container. Works with Postgres, file system, or FoundationDB (enterprise). Full dashboard included.
          </p>
          <div className='mb-6 font-mono text-xs text-zinc-500'>
            <div className='flex gap-2'>
              <span className='select-none text-zinc-600'>$</span>
              <span>docker run -p 6420:6420 rivetdev/engine</span>
            </div>
          </div>
          <div className='mt-auto'>
            <a
              href='/docs/self-hosting'
              className='inline-flex items-center justify-center whitespace-nowrap rounded-md border border-white/10 px-4 py-2 text-sm text-zinc-300 transition-colors hover:border-white/20 hover:text-white'
            >
              View Self-Hosting Docs
            </a>
          </div>
        </div>

        {/* Card 3: Rivet Cloud */}
        <div className='flex flex-col border-t border-white/10 pt-6'>
          <div className='mb-3'>
            <img className='h-5 w-5 opacity-50' src={imgLogo.src} alt='Rivet' />
          </div>
          <h3 className='mb-2 text-base font-normal text-white'>Rivet Cloud</h3>
          <p className='mb-6 text-sm leading-relaxed text-zinc-500'>
            Fully managed Actors and agentOS. Global edge network. Connects to your existing cloud — Vercel, Railway, AWS, wherever you already deploy.
          </p>
          <ul className='mb-6 space-y-1'>
            {['Global Edge Network', 'Scales Seamlessly', 'Connects To Your Cloud'].map(item => (
              <li key={item} className='flex items-center gap-2 text-xs text-zinc-500'>
                <Check className='h-3 w-3' /> {item}
              </li>
            ))}
          </ul>
          <div className='mt-auto'>
            <a
              href='https://hub.rivet.dev'
              target='_blank'
              rel='noopener noreferrer'
              className='selection-dark inline-flex items-center justify-center gap-2 rounded-md bg-white px-4 py-2 text-sm font-medium text-black transition-colors hover:bg-zinc-200'
            >
              Sign Up
            </a>
          </div>
        </div>
      </motion.div>

      {/* Open Source callout */}
      <motion.div
        initial={{ opacity: 0, y: 20 }}
        whileInView={{ opacity: 1, y: 0 }}
        viewport={{ once: true }}
        transition={{ duration: 0.5 }}
        className='mt-24 text-center'
      >
        <h3 className='mb-3 text-lg font-normal tracking-tight text-white md:text-xl'>
          Open source, permissively licensed.
        </h3>
        <p className='mx-auto max-w-md text-sm leading-relaxed text-zinc-500'>
          Self-hosting matters for enterprise deployments, cloud portability, and avoiding vendor lock-in. Licensed Apache 2.0.
        </p>
        <a
          href='https://github.com/rivet-dev/rivet'
          target='_blank'
          rel='noopener noreferrer'
          className='mt-5 inline-flex items-center gap-2 rounded-md border border-white/10 px-4 py-2 text-sm text-zinc-300 transition-colors hover:border-white/20 hover:text-white'
        >
          <Github className='h-4 w-4' />
          View on GitHub
        </a>
      </motion.div>
    </div>
  </section>
);
