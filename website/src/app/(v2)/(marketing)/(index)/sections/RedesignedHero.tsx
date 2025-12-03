'use client';

import { Terminal, ArrowRight, Check } from 'lucide-react';
import { motion } from 'framer-motion';
import Link from 'next/link';
import { useState } from 'react';

const CopyInstallButton = () => {
  const [copied, setCopied] = useState(false);

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText('npm install rivetkit');
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch (err) {
      console.error('Failed to copy:', err);
    }
  };

  return (
    <button
      onClick={handleCopy}
      className='font-v2 inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md border border-white/10 bg-white/5 px-4 py-2 text-sm text-white subpixel-antialiased shadow-sm transition-colors hover:border-white/20'
    >
      {copied ? <Check className='h-4 w-4' /> : <Terminal className='h-4 w-4' />}
      npm install rivetkit
    </button>
  );
};

const CodeBlock = ({ code, fileName = 'actor.ts' }) => {
  return (
    <div className='group relative overflow-hidden rounded-xl border border-white/10 bg-zinc-900/50 shadow-2xl backdrop-blur-xl'>
      <div className='flex items-center justify-between border-b border-white/5 bg-white/5 px-4 py-3'>
        <div className='flex items-center gap-2'>
          <div className='h-3 w-3 rounded-full border border-zinc-500/50 bg-zinc-500/20' />
          <div className='h-3 w-3 rounded-full border border-zinc-500/50 bg-zinc-500/20' />
          <div className='h-3 w-3 rounded-full border border-zinc-500/50 bg-zinc-500/20' />
        </div>
        <div className='font-mono text-xs text-zinc-500'>{fileName}</div>
      </div>
      <div className='scrollbar-hide overflow-x-auto p-4'>
        <pre className='font-mono text-sm leading-relaxed text-zinc-300'>
          <code>
            {code.split('\n').map((line, i) => (
              <div key={i} className='table-row'>
                <span className='table-cell w-8 select-none pr-4 text-right text-zinc-700'>{i + 1}</span>
                <span className='table-cell'>
                  {(() => {
                    // Simple custom tokenizer for this snippet
                    const tokens = [];
                    let current = line;

                    // Handle comments first (consume rest of line)
                    const commentIndex = current.indexOf('//');
                    let comment = '';
                    if (commentIndex !== -1) {
                      comment = current.slice(commentIndex);
                      current = current.slice(0, commentIndex);
                    }

                    // Split remaining code by delimiters but keep them
                    // Note: this is still basic but better than before
                    const parts = current
                      .split(/([a-zA-Z0-9_$]+|"[^"]*"|'[^']*'|\s+|[(){},.;:[\]])/g)
                      .filter(Boolean);

                    parts.forEach((part, j) => {
                      const trimmed = part.trim();

                      // Keywords
                      if (
                        [
                          'import',
                          'from',
                          'export',
                          'const',
                          'return',
                          'async',
                          'await',
                          'function'
                        ].includes(trimmed)
                      ) {
                        tokens.push(
                          <span key={j} className='text-purple-400'>
                            {part}
                          </span>
                        );
                      }
                      // Functions & Special Rivet Terms
                      else if (['actor', 'broadcast'].includes(trimmed)) {
                        tokens.push(
                          <span key={j} className='text-blue-400'>
                            {part}
                          </span>
                        );
                      }
                      // Object Keys / Properties / Methods
                      else if (
                        ['state', 'actions', 'sendMessage', 'user', 'text', 'messages', 'push'].includes(trimmed)
                      ) {
                        tokens.push(
                          <span key={j} className='text-blue-300'>
                            {part}
                          </span>
                        );
                      }
                      // Strings
                      else if (part.startsWith('"') || part.startsWith("'")) {
                        tokens.push(
                          <span key={j} className='text-[#FF4500]'>
                            {part}
                          </span>
                        );
                      }
                      // Numbers
                      else if (!isNaN(Number(trimmed)) && trimmed !== '') {
                        tokens.push(
                          <span key={j} className='text-emerald-400'>
                            {part}
                          </span>
                        );
                      }
                      // Default (punctuation, variables like 'c', etc)
                      else {
                        tokens.push(
                          <span key={j} className='text-zinc-300'>
                            {part}
                          </span>
                        );
                      }
                    });

                    if (comment) {
                      tokens.push(
                        <span key='comment' className='text-zinc-500'>
                          {comment}
                        </span>
                      );
                    }

                    return tokens;
                  })()}
                </span>
              </div>
            ))}
          </code>
        </pre>
      </div>
    </div>
  );
};

interface RedesignedHeroProps {
  latestChangelogTitle: string;
}

export const RedesignedHero = ({ latestChangelogTitle }: RedesignedHeroProps) => (
  <section className='relative overflow-hidden pb-20 pt-32 md:pb-32 md:pt-48'>
    <div className='pointer-events-none absolute left-1/2 top-0 h-[500px] w-[1000px] -translate-x-1/2 rounded-full bg-white/[0.02] blur-[100px]' />

    <div className='relative z-10 mx-auto max-w-7xl px-6'>
      <div className='flex flex-col items-center gap-16 lg:flex-row'>
        <div className='max-w-2xl flex-1'>
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.5 }}
          >
            <Link
              href='/changelog'
              className='mb-8 inline-flex items-center gap-2 rounded-full border border-white/10 bg-white/5 px-3 py-1 text-xs font-medium text-zinc-400 transition-colors hover:border-white/20'
            >
              <span className='h-2 w-2 animate-pulse rounded-full bg-[#FF4500]' />
              {latestChangelogTitle}
              <ArrowRight className='ml-1 h-3 w-3' />
            </Link>
          </motion.div>

          <motion.h1
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.5, delay: 0.1 }}
            className='mb-6 text-5xl font-medium leading-[1.1] tracking-tighter text-white md:text-7xl'
          >
            Stateful Backends. <br />
            <span className='bg-gradient-to-b from-zinc-200 to-zinc-500 bg-clip-text text-transparent'>
              Finally Solved.
            </span>
          </motion.h1>

          <motion.p
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.5, delay: 0.2 }}
            className='mb-8 max-w-lg text-lg leading-relaxed text-zinc-400 md:text-xl'
          >
            Rivet is open-source infrastructure for long-lived, in-memory processes called Actors. It's what
            you reach for when you hit the limitations of HTTP, databases, or queues.
          </motion.p>

          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.5, delay: 0.3 }}
            className='flex flex-col items-center gap-4 sm:flex-row'
          >
            <Link
              href='/docs'
              className='font-v2 inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md border border-white/10 bg-white px-4 py-2 text-sm text-black subpixel-antialiased shadow-sm transition-colors hover:bg-zinc-200'
            >
              Start Building
              <ArrowRight className='h-4 w-4' />
            </Link>
            <CopyInstallButton />
          </motion.div>
        </div>

        <div className='w-full max-w-xl flex-1'>
          <motion.div
            initial={{ opacity: 0, scale: 0.95 }}
            animate={{ opacity: 1, scale: 1 }}
            transition={{ duration: 0.7, delay: 0.4, ease: [0.21, 0.47, 0.32, 0.98] }}
            className='relative'
          >
            <div className='absolute -inset-1 rounded-xl bg-gradient-to-r from-zinc-700 to-zinc-800 opacity-20 blur' />
            <CodeBlock
              code={`import { actor } from "rivetkit";

export const chatRoom = actor({
  // In-memory, persisted state
  state: { messages: [] },

  // Type-safe RPC
  actions: {
    sendMessage: (c, user, text) => {
      // High performance writes
      c.state.messages.push({ user, text });

      // Realtime built-in
      c.broadcast("newMessage", { user, text });
    },
  },
});`}
            />
          </motion.div>
        </div>
      </div>
    </div>
  </section>
);


