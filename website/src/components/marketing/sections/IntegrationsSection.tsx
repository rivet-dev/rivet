'use client';

import { Box, LayoutGrid, Terminal, Wrench } from 'lucide-react';
import { motion } from 'framer-motion';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { deployOptions } from '@rivetkit/example-registry';

const frameworks = [
  { name: 'React', href: '/docs/clients/react' },
  { name: 'Next.js', href: '/docs/clients/next-js' },
  { name: 'Svelte', href: 'https://github.com/rivet-dev/rivetkit/pull/1172', external: true },
  { name: 'Hono', href: 'https://github.com/rivet-dev/rivet/tree/main/examples/hono', external: true },
  { name: 'Express', href: 'https://github.com/rivet-dev/rivet/tree/main/examples/express', external: true },
  { name: 'Elysia', href: 'https://github.com/rivet-dev/rivet/tree/main/examples/elysia', external: true },
  { name: 'tRPC', href: 'https://github.com/rivet-dev/rivet/tree/main/examples/trpc', external: true },
];

export const IntegrationsSection = () => (
  <section className='relative overflow-hidden border-t border-white/5 bg-zinc-900/20 py-32'>
    <div className='relative z-10 mx-auto max-w-7xl px-6'>
      <div className='mb-16 flex flex-col items-center justify-between gap-12 md:flex-row'>
        <motion.div
          initial={{ opacity: 0, x: -20 }}
          whileInView={{ opacity: 1, x: 0 }}
          viewport={{ once: true }}
          transition={{ duration: 0.5 }}
          className='max-w-xl'
        >
          <h2 className='mb-6 text-3xl font-medium tracking-tight text-white md:text-5xl'>Runs where you do.</h2>
          <p className='text-lg leading-relaxed text-zinc-400'>
            Rivet Actors integrate with your existing infrastructure, frameworks, runtimes, and tools.
          </p>
        </motion.div>
      </div>

      <div className='grid grid-cols-1 gap-6 md:grid-cols-2 lg:grid-cols-4'>
        {/* Category 1: Infrastructure (Blue) */}
        <div className='group relative flex flex-col gap-4 overflow-hidden rounded-2xl border border-white/5 bg-black/50 p-6 backdrop-blur-sm'>
          {/* Top Shine Highlight - existing */}
          <div className='absolute left-0 right-0 top-0 z-10 h-[1px] bg-gradient-to-r from-transparent via-white/20 to-transparent' />

          {/* NEW: Top Left Reflection/Glow (Reduced opacity and soft fade) */}
          <div className='pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_top_left,rgba(59,130,246,0.15)_0%,transparent_50%)] opacity-0 transition-opacity duration-500 group-hover:opacity-100' />
          {/* NEW: Sharp Edge Highlight (Masked to Fade - Fixed Clipping) */}
          <div className='pointer-events-none absolute left-0 top-0 z-20 h-24 w-24 rounded-tl-2xl border-l border-t border-blue-500 opacity-0 transition-opacity duration-500 [mask-image:linear-gradient(135deg,black_0%,transparent_50%)] group-hover:opacity-100' />

          <div className='relative z-10 mb-2 flex items-center gap-3'>
            {/* Updated Icon Container */}
            <div className='rounded bg-blue-500/10 p-2 text-blue-400 transition-all duration-500 group-hover:bg-blue-500/20 group-hover:shadow-[0_0_15px_rgba(59,130,246,0.5)]'>
              <Box className='h-4 w-4' />
            </div>
            <h4 className='text-sm font-medium uppercase tracking-wider text-white'>Infrastructure</h4>
          </div>
          <div className='relative z-10 flex flex-wrap gap-2'>
            {deployOptions.map(({ displayName, shortTitle, href }) => (
              <a
                key={displayName}
                href={href}
                className='group/item flex cursor-pointer items-center gap-1.5 rounded-md border border-white/5 bg-zinc-800/50 px-3 py-1.5 text-xs text-zinc-300 transition-colors hover:border-white/20 hover:text-white'
              >
                {shortTitle || displayName}
              </a>
            ))}
          </div>
        </div>

        {/* Category 2: Frameworks (Purple) */}
        <div className='group relative flex flex-col gap-4 overflow-hidden rounded-2xl border border-white/5 bg-black/50 p-6 backdrop-blur-sm'>
          {/* Top Shine Highlight - existing */}
          <div className='absolute left-0 right-0 top-0 z-10 h-[1px] bg-gradient-to-r from-transparent via-white/20 to-transparent' />

          {/* NEW: Top Left Reflection/Glow (Reduced opacity and soft fade) */}
          <div className='pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_top_left,rgba(168,85,247,0.15)_0%,transparent_50%)] opacity-0 transition-opacity duration-500 group-hover:opacity-100' />
          {/* NEW: Sharp Edge Highlight (Masked to Fade - Fixed Clipping) */}
          <div className='pointer-events-none absolute left-0 top-0 z-20 h-24 w-24 rounded-tl-2xl border-l border-t border-purple-500 opacity-0 transition-opacity duration-500 [mask-image:linear-gradient(135deg,black_0%,transparent_50%)] group-hover:opacity-100' />

          <div className='relative z-10 mb-2 flex items-center gap-3'>
            {/* Updated Icon Container */}
            <div className='rounded bg-purple-500/10 p-2 text-purple-400 transition-all duration-500 group-hover:bg-purple-500/20 group-hover:shadow-[0_0_15px_rgba(168,85,247,0.5)]'>
              <LayoutGrid className='h-4 w-4' />
            </div>
            <h4 className='text-sm font-medium uppercase tracking-wider text-white'>Frameworks</h4>
          </div>
          <div className='relative z-10 flex flex-wrap gap-2'>
            {frameworks.map(tech => (
              <a
                key={tech.name}
                href={tech.href}
                {...(tech.external ? { target: '_blank', rel: 'noopener noreferrer' } : {})}
                className='cursor-pointer rounded-md border border-white/5 bg-zinc-800/50 px-3 py-1.5 text-xs text-zinc-300 transition-colors hover:border-white/20 hover:text-white'
              >
                {tech.name}
              </a>
            ))}
          </div>
        </div>

        {/* Category 3: Runtimes (Yellow) */}
        <div className='group relative flex flex-col gap-4 overflow-hidden rounded-2xl border border-white/5 bg-black/50 p-6 backdrop-blur-sm'>
          {/* Top Shine Highlight - existing */}
          <div className='absolute left-0 right-0 top-0 z-10 h-[1px] bg-gradient-to-r from-transparent via-white/20 to-transparent' />

          {/* NEW: Top Left Reflection/Glow (Reduced opacity and soft fade) */}
          <div className='pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_top_left,rgba(234,179,8,0.15)_0%,transparent_50%)] opacity-0 transition-opacity duration-500 group-hover:opacity-100' />
          {/* NEW: Sharp Edge Highlight (Masked to Fade - Fixed Clipping) */}
          <div className='pointer-events-none absolute left-0 top-0 z-20 h-24 w-24 rounded-tl-2xl border-l border-t border-yellow-500 opacity-0 transition-opacity duration-500 [mask-image:linear-gradient(135deg,black_0%,transparent_50%)] group-hover:opacity-100' />

          <div className='relative z-10 mb-2 flex items-center gap-3'>
            {/* Updated Icon Container */}
            <div className='rounded bg-yellow-500/10 p-2 text-yellow-400 transition-all duration-500 group-hover:bg-yellow-500/20 group-hover:shadow-[0_0_15px_rgba(234,179,8,0.5)]'>
              <Terminal className='h-4 w-4' />
            </div>
            <h4 className='text-sm font-medium uppercase tracking-wider text-white'>Runtimes</h4>
          </div>
          <div className='relative z-10 flex flex-wrap gap-2'>
            {[
              { name: 'Node.js', href: '/docs/actors/quickstart/backend' },
              { name: 'Bun', href: '/docs/actors/quickstart/backend' },
              { name: 'Deno', href: 'https://github.com/rivet-dev/rivet/tree/main/examples/deno', external: true },
              { name: 'Cloudflare Workers', href: '/docs/actors/quickstart/cloudflare-workers' }
            ].map(tech => (
              <a
                key={tech.name}
                href={tech.href}
                {...(tech.external ? { target: '_blank', rel: 'noopener noreferrer' } : {})}
                className='cursor-pointer rounded-md border border-white/5 bg-zinc-800/50 px-3 py-1.5 text-xs text-zinc-300 transition-colors hover:border-white/20 hover:text-white'
              >
                {tech.name}
              </a>
            ))}
          </div>
        </div>

        {/* Category 4: Tools (Emerald) */}
        <div className='group relative flex flex-col gap-4 overflow-hidden rounded-2xl border border-white/5 bg-black/50 p-6 backdrop-blur-sm'>
          {/* Top Shine Highlight - existing */}
          <div className='absolute left-0 right-0 top-0 z-10 h-[1px] bg-gradient-to-r from-transparent via-white/20 to-transparent' />

          {/* NEW: Top Left Reflection/Glow (Reduced opacity and soft fade) */}
          <div className='pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_top_left,rgba(16,185,129,0.15)_0%,transparent_50%)] opacity-0 transition-opacity duration-500 group-hover:opacity-100' />
          {/* NEW: Sharp Edge Highlight (Masked to Fade - Fixed Clipping) */}
          <div className='pointer-events-none absolute left-0 top-0 z-20 h-24 w-24 rounded-tl-2xl border-l border-t border-emerald-500 opacity-0 transition-opacity duration-500 [mask-image:linear-gradient(135deg,black_0%,transparent_50%)] group-hover:opacity-100' />

          <div className='relative z-10 mb-2 flex items-center gap-3'>
            {/* Updated Icon Container */}
            <div className='rounded bg-emerald-500/10 p-2 text-emerald-400 transition-all duration-500 group-hover:bg-emerald-500/20 group-hover:shadow-[0_0_15px_rgba(16,185,129,0.5)]'>
              <Wrench className='h-4 w-4' />
            </div>
            <h4 className='text-sm font-medium uppercase tracking-wider text-white'>Tools</h4>
          </div>
          <div className='relative z-10 flex flex-wrap gap-2'>
            {[
              { name: 'Vitest', href: '/docs/actors/testing' },
              { name: 'Pino', href: '/docs/general/logging' },
              { name: 'AI SDK', href: 'https://github.com/rivet-dev/rivet/tree/main/examples/ai-agent', external: true },
              { name: 'OpenAPI', href: 'https://github.com/rivet-dev/rivet/tree/main/rivetkit-openapi', external: true },
              { name: 'AsyncAPI', href: 'https://github.com/rivet-dev/rivet/tree/main/rivetkit-asyncapi', external: true },
              { name: 'TypeDoc', href: '/typedoc', external: true }
            ].map(tech => (
              <a
                key={tech.name}
                href={tech.href}
                {...(tech.external ? { target: '_blank', rel: 'noopener noreferrer' } : {})}
                className='cursor-pointer rounded-md border border-white/5 bg-zinc-800/50 px-3 py-1.5 text-xs text-zinc-300 transition-colors hover:border-white/20 hover:text-white'
              >
                {tech.name}
              </a>
            ))}
          </div>
        </div>
      </div>
    </div>
  </section>
);
