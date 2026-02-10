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
  <section className='relative overflow-hidden border-t border-white/5 py-48'>
    <div className='relative z-10 mx-auto max-w-7xl px-6'>
      <div className='mb-12'>
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          transition={{ duration: 0.5 }}
          className='max-w-xl'
        >
          <h2 className='mb-2 text-2xl font-normal tracking-tight text-white md:text-4xl'>Runs where you do.</h2>
          <p className='text-base leading-relaxed text-zinc-500'>
            Rivet Actors integrate with your existing infrastructure, frameworks, runtimes, and tools.
          </p>
        </motion.div>
      </div>

      <div className='grid grid-cols-1 gap-8 md:grid-cols-2 lg:grid-cols-4'>
        {/* Category 1: Infrastructure */}
        <div className='border-t border-white/10 pt-6'>
          <div className='mb-4 flex items-center gap-3'>
            <Box className='h-4 w-4 text-zinc-400' />
            <h4 className='text-sm font-medium uppercase tracking-wider text-white'>Infrastructure</h4>
          </div>
          <div className='flex flex-wrap gap-2'>
            {deployOptions.map(({ displayName, shortTitle, href }) => (
              <a
                key={displayName}
                href={href}
                className='rounded-md border border-white/5 px-2 py-1 text-xs text-zinc-400 transition-colors hover:border-white/20 hover:text-white'
              >
                {shortTitle || displayName}
              </a>
            ))}
          </div>
        </div>

        {/* Category 2: Frameworks */}
        <div className='border-t border-white/10 pt-6'>
          <div className='mb-4 flex items-center gap-3'>
            <LayoutGrid className='h-4 w-4 text-zinc-400' />
            <h4 className='text-sm font-medium uppercase tracking-wider text-white'>Frameworks</h4>
          </div>
          <div className='flex flex-wrap gap-2'>
            {frameworks.map(tech => (
              <a
                key={tech.name}
                href={tech.href}
                {...(tech.external ? { target: '_blank', rel: 'noopener noreferrer' } : {})}
                className='rounded-md border border-white/5 px-2 py-1 text-xs text-zinc-400 transition-colors hover:border-white/20 hover:text-white'
              >
                {tech.name}
              </a>
            ))}
          </div>
        </div>

        {/* Category 3: Runtimes */}
        <div className='border-t border-white/10 pt-6'>
          <div className='mb-4 flex items-center gap-3'>
            <Terminal className='h-4 w-4 text-zinc-400' />
            <h4 className='text-sm font-medium uppercase tracking-wider text-white'>Runtimes</h4>
          </div>
          <div className='flex flex-wrap gap-2'>
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
                className='rounded-md border border-white/5 px-2 py-1 text-xs text-zinc-400 transition-colors hover:border-white/20 hover:text-white'
              >
                {tech.name}
              </a>
            ))}
          </div>
        </div>

        {/* Category 4: Tools */}
        <div className='border-t border-white/10 pt-6'>
          <div className='mb-4 flex items-center gap-3'>
            <Wrench className='h-4 w-4 text-zinc-400' />
            <h4 className='text-sm font-medium uppercase tracking-wider text-white'>Tools</h4>
          </div>
          <div className='flex flex-wrap gap-2'>
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
                className='rounded-md border border-white/5 px-2 py-1 text-xs text-zinc-400 transition-colors hover:border-white/20 hover:text-white'
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
