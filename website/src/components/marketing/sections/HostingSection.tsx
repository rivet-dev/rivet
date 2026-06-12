'use client';

import { Package, Server, Check } from 'lucide-react';
import { motion } from 'framer-motion';
import { deployOptions } from '@rivetkit/shared-data';
import imgLogo from '@/images/rivet-logos/icon-white.svg';
import { SECTION_H2_CLASS, SUBTITLE_CLASS, EYEBROW_CLASS } from '../typography';
import { Eyebrow } from '../editorial/Eyebrow';
import { InkChip } from '../editorial/InkPanel';

export const HostingSection = () => (
  <section className='border-t border-ink/10 py-16 md:py-32'>
    <div className='mx-auto max-w-7xl px-6'>
      <div className='mb-12'>
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          transition={{ duration: 0.5 }}
        >
          <Eyebrow index='04' label='Run anywhere' className='mb-4' />
          <h2 className={`mb-2 ${SECTION_H2_CLASS}`}>Start local. Scale to millions.</h2>
        </motion.div>
        <motion.p
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          transition={{ duration: 0.5, delay: 0.1 }}
          className={`max-w-xl ${SUBTITLE_CLASS}`}
        >
          A library in development, a platform in production. Your backend keeps deploying wherever it already does — Rivet connects to it.
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
        <div className='flex flex-col border-t border-ink/10 pt-6'>
          <div className='mb-3 flex items-center gap-3'>
            <Package className='h-4 w-4 text-olive' />
            <span className={EYEBROW_CLASS}>
              <span className='text-ink-faint'>№ 1</span>
            </span>
          </div>
          <h3 className='mb-2 text-base font-medium text-ink'>Just a Library</h3>
          <p className='mb-6 text-sm leading-relaxed text-ink-soft'>
            Install a package and run locally. No servers, no infrastructure. Actors run in your process during development.
          </p>
          <InkChip command='npm install rivetkit' className='mb-6' />
          <div className='mt-auto'>
            <a
              href='/docs/actors/quickstart'
              className='inline-flex items-center justify-center whitespace-nowrap rounded-md border border-ink/20 px-4 py-2 text-sm text-ink-soft transition-colors hover:border-ink/40 hover:text-ink'
            >
              Get Started
            </a>
          </div>
        </div>

        {/* Card 2: Rivet Cloud */}
        <div className='flex flex-col border-t border-ink/10 pt-6'>
          <div className='mb-3 flex items-center gap-3'>
            <img className='h-5 w-5' src={imgLogo.src} alt='Rivet' />
            <span className={EYEBROW_CLASS}>
              <span className='text-ink-faint'>№ 2</span>
            </span>
          </div>
          <h3 className='mb-2 text-base font-medium text-ink'>Rivet Cloud</h3>
          <p className='mb-6 text-sm leading-relaxed text-ink-soft'>
            Fully managed Actors and agentOS. Global edge network. Connects to your existing cloud — Vercel, Railway, AWS, wherever you already deploy.
          </p>
          <ul className='mb-6 space-y-1'>
            {['Global Edge Network', 'Scales Seamlessly', 'Connects To Your Cloud'].map(item => (
              <li key={item} className='flex items-center gap-2 text-xs text-ink-soft'>
                <Check className='h-3 w-3 text-pine' /> {item}
              </li>
            ))}
          </ul>
          <div className='mt-auto'>
            <a
              href='https://dashboard.rivet.dev'
              target='_blank'
              rel='noopener noreferrer'
              className='inline-flex items-center justify-center gap-2 rounded-md bg-ink px-4 py-2 text-sm font-medium text-cream transition-colors hover:bg-ink/85'
            >
              Sign Up
            </a>
          </div>
        </div>

        {/* Card 3: Self-Host */}
        <div className='flex flex-col border-t border-ink/10 pt-6'>
          <div className='mb-3 flex items-center gap-3'>
            <Server className='h-4 w-4 text-olive' />
            <span className={EYEBROW_CLASS}>
              <span className='text-ink-faint'>№ 3</span>
            </span>
          </div>
          <h3 className='mb-2 text-base font-medium text-ink'>Self-Host</h3>
          <p className='mb-6 text-sm leading-relaxed text-ink-soft'>
            Single Rust binary or Docker container. Works with Postgres, file system, or FoundationDB (enterprise). Full dashboard included.
          </p>
          <InkChip command='docker run -p 6420:6420 rivetdev/engine' className='mb-6' />
          <div className='mt-auto'>
            <a
              href='/docs/self-hosting'
              className='inline-flex items-center justify-center whitespace-nowrap rounded-md border border-ink/20 px-4 py-2 text-sm text-ink-soft transition-colors hover:border-ink/40 hover:text-ink'
            >
              View Self-Hosting Docs
            </a>
          </div>
        </div>
      </motion.div>

      <motion.div
        initial={{ opacity: 0, y: 20 }}
        whileInView={{ opacity: 1, y: 0 }}
        viewport={{ once: true }}
        transition={{ duration: 0.5, delay: 0.1 }}
        className='mt-12 flex flex-wrap items-center gap-x-5 gap-y-2.5 border-t border-ink/10 pt-6'
      >
        <span className='font-mono text-[11px] uppercase tracking-[0.16em] text-ink-faint'>Your backend deploys to</span>
        {deployOptions.map(({ displayName, shortTitle, href }) => (
          <a
            key={displayName}
            href={href}
            className='text-sm text-ink-soft underline decoration-ink/20 underline-offset-4 transition-colors hover:text-ink hover:decoration-pine'
          >
            {shortTitle || displayName}
          </a>
        ))}
      </motion.div>

    </div>
  </section>
);
