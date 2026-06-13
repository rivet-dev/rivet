'use client';

import { useState } from 'react';
import { Package, Server, Check, Copy } from 'lucide-react';
import { motion } from 'framer-motion';
import { deployOptions } from '@rivetkit/shared-data';
import imgLogo from '@/images/rivet-logos/icon-white.svg';
import { SECTION_H2_CLASS, SUBTITLE_CLASS } from '../typography';
import { GLOW_PILL_CLASS, handleGlowPillMouseMove } from '../glowPill';

const DEPLOY_PILL_CLASS =
  `${GLOW_PILL_CLASS} inline-flex items-center rounded-full border border-ink/12 bg-paper/45 px-2.5 py-1 text-[13px] text-ink-soft transition-colors hover:border-ink/25 hover:text-ink`;

// Compact terminal command block, matching the command blocks on the cookbook
// pages: flat ink with a hairline border, monospace, and an absolute copy
// button. Commands stay on one line and scroll horizontally rather than wrap.
const COMMAND_BLOCK_CLASS =
  'selection-paper relative rounded-md border border-ink/20 bg-ink p-3 pr-11 font-mono text-[11px] leading-relaxed text-cream/85';

const CARD_CLASS = 'relative flex h-full flex-col border border-ink/10 bg-white/55 p-6 md:p-8';
const CARD_TITLE_CLASS = 'text-base font-medium tracking-tight text-ink';
const BUTTON_BASE =
  'inline-flex h-10 w-full items-center justify-center gap-2 whitespace-nowrap rounded-md px-4 text-sm font-medium transition-colors';
const GHOST_BUTTON_CLASS = `${BUTTON_BASE} border border-ink/15 text-ink-soft hover:border-ink/40 hover:text-ink`;
const WHITE_BUTTON_CLASS = `${BUTTON_BASE} border border-ink/15 bg-white text-ink hover:border-ink/30`;

// Terminal command block. Mirrors the cookbook command-block markup: a
// horizontally-scrollable single-line command with an absolute copy button.
const TerminalCommand = ({ command }: { command: string }) => {
  const [copied, setCopied] = useState(false);

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(command);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch (err) {
      console.error('Failed to copy command:', err);
    }
  };

  return (
    <div className={COMMAND_BLOCK_CLASS}>
      <div className='scrollbar-hide overflow-x-auto'>
        <code className='select-all whitespace-nowrap'>{command}</code>
      </div>
      <button
        type='button'
        onClick={handleCopy}
        aria-label={copied ? 'Copied' : 'Copy command'}
        className='absolute right-1.5 top-1.5 rounded p-1.5 text-cream/50 transition-colors hover:bg-cream/10 hover:text-cream'
      >
        {copied ? <Check className='h-4 w-4 text-sage' /> : <Copy className='h-4 w-4' />}
      </button>
    </div>
  );
};

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
        className='grid grid-cols-1 gap-6 md:grid-cols-3 md:items-stretch'
      >
        {/* Card 1: Just a Library */}
        <div className={CARD_CLASS}>
          <div className='mb-3 flex h-6 items-center gap-2.5'>
            <Package className='h-4 w-4 text-olive' />
            <h3 className={CARD_TITLE_CLASS}>Just a Library</h3>
          </div>
          <p className='text-sm leading-relaxed text-ink-soft'>
            Install a package and run locally. No servers, no infrastructure. Actors run in your process during development.
          </p>
          <div className='flex-1' />
          <div className='mt-6 border-t border-ink/10' />
          <div className='flex flex-col gap-4 pt-6'>
            <TerminalCommand command='npm install rivetkit' />
            <a href='/docs/actors/quickstart' className={GHOST_BUTTON_CLASS}>
              Get Started
            </a>
          </div>
        </div>

        {/* Card 2: Rivet Cloud (primary) */}
        <div className={`${CARD_CLASS} border-ink/20`}>
          <div className='mb-3 flex h-6 items-center gap-2.5'>
            <span className='flex h-4 w-4 items-center justify-center'>
              <img className='h-4 w-4' src={imgLogo.src} alt='Rivet' />
            </span>
            <h3 className={CARD_TITLE_CLASS}>Rivet Cloud</h3>
          </div>
          <p className='text-sm leading-relaxed text-ink-soft'>
            Fully managed Actors and agentOS on a global edge network. Connects to your existing cloud — Vercel, Railway, AWS, wherever you deploy.
          </p>
          <div className='flex-1' />
          <a
            href='https://dashboard.rivet.dev'
            target='_blank'
            rel='noopener noreferrer'
            className={WHITE_BUTTON_CLASS}
          >
            Sign Up
          </a>
        </div>

        {/* Card 3: Self-Host */}
        <div className={CARD_CLASS}>
          <div className='mb-3 flex h-6 items-center gap-2.5'>
            <Server className='h-4 w-4 text-olive' />
            <h3 className={CARD_TITLE_CLASS}>Self-Host</h3>
          </div>
          <p className='text-sm leading-relaxed text-ink-soft'>
            Single Rust binary or Docker container. Works with Postgres, file system, or FoundationDB (enterprise). Full dashboard included.
          </p>
          <div className='flex-1' />
          <div className='mt-6 border-t border-ink/10' />
          <div className='flex flex-col gap-4 pt-6'>
            <TerminalCommand command='docker run -p 6420:6420 rivetdev/engine' />
            <a href='/docs/self-hosting' className={GHOST_BUTTON_CLASS}>
              Self-Hosting Docs
            </a>
          </div>
        </div>
      </motion.div>

      <motion.div
        initial={{ opacity: 0, y: 20 }}
        whileInView={{ opacity: 1, y: 0 }}
        viewport={{ once: true }}
        transition={{ duration: 0.5, delay: 0.1 }}
        className='mt-10 flex flex-wrap items-center gap-x-2 gap-y-2 border-t border-ink/10 pt-6'
      >
        <span className='mr-3 font-mono text-[11px] uppercase tracking-[0.16em] text-ink-faint'>Your backend deploys to</span>
        {deployOptions.map(({ displayName, shortTitle, href }) => (
          <a key={displayName} href={href} onMouseMove={handleGlowPillMouseMove} className={DEPLOY_PILL_CLASS}>
            {shortTitle || displayName}
          </a>
        ))}
      </motion.div>
    </div>
  </section>
);
