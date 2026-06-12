'use client';

import { LayoutGrid, Terminal, Wrench } from 'lucide-react';
import { motion } from 'framer-motion';
import { SECTION_H2_CLASS, SUBTITLE_CLASS } from '../typography';
import { Eyebrow } from '../editorial/Eyebrow';

const frameworks = [
  { name: 'React', href: '/docs/clients/react' },
  { name: 'Next.js', href: '/docs/clients/next-js' },
  { name: 'Svelte', href: '/docs/clients/javascript' },
  { name: 'Hono', href: 'https://github.com/rivet-dev/rivet/tree/main/examples/hono', external: true },
  { name: 'Express', href: 'https://github.com/rivet-dev/rivet/tree/main/examples/express', external: true },
  { name: 'Elysia', href: 'https://github.com/rivet-dev/rivet/tree/main/examples/elysia', external: true },
  { name: 'tRPC', href: 'https://github.com/rivet-dev/rivet/tree/main/examples/trpc', external: true },
];

export const IntegrationsSection = () => (
  <section className='relative overflow-hidden border-t border-ink/10 py-16 md:py-32'>
    <div className='relative z-10 mx-auto max-w-7xl px-6'>
      <div className='mb-12'>
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          transition={{ duration: 0.5 }}
          className='max-w-xl'
        >
          <Eyebrow index='03' label='Compatibility' className='mb-4' />
          <h2 className={`mb-2 ${SECTION_H2_CLASS}`}>Works with your stack.</h2>
          <p className={SUBTITLE_CLASS}>
            Standard Node.js, Bun, and Deno. Your frameworks, your tools. No custom runtime, no rewrite.
          </p>
        </motion.div>
      </div>

      <div className='grid grid-cols-1 gap-8 md:grid-cols-3'>
        {/* Category 1: Frameworks */}
        <div className='border-t border-ink/10 pt-6'>
          <div className='mb-4 flex items-center gap-3'>
            <LayoutGrid className='h-4 w-4 text-olive' />
            <h4 className='font-mono text-[11px] font-medium uppercase tracking-[0.16em] text-ink-faint'>Frameworks</h4>
          </div>
          <div className='flex flex-wrap gap-x-5 gap-y-2.5'>
            {frameworks.map(tech => (
              <a
                key={tech.name}
                href={tech.href}
                {...(tech.external ? { target: '_blank', rel: 'noopener noreferrer' } : {})}
                className='text-sm text-ink-soft underline decoration-ink/20 underline-offset-4 transition-colors hover:text-ink hover:decoration-pine'
              >
                {tech.name}
              </a>
            ))}
          </div>
        </div>

        {/* Category 2: Runtimes */}
        <div className='border-t border-ink/10 pt-6'>
          <div className='mb-4 flex items-center gap-3'>
            <Terminal className='h-4 w-4 text-olive' />
            <h4 className='font-mono text-[11px] font-medium uppercase tracking-[0.16em] text-ink-faint'>Runtimes</h4>
          </div>
          <div className='flex flex-wrap gap-x-5 gap-y-2.5'>
            {[
              { name: 'Node.js', href: '/docs/actors/quickstart/backend' },
              { name: 'Bun', href: '/docs/actors/quickstart/backend' },
              { name: 'Deno', href: 'https://github.com/rivet-dev/rivet/tree/main/examples/deno', external: true }
            ].map(tech => (
              <a
                key={tech.name}
                href={tech.href}
                {...(tech.external ? { target: '_blank', rel: 'noopener noreferrer' } : {})}
                className='text-sm text-ink-soft underline decoration-ink/20 underline-offset-4 transition-colors hover:text-ink hover:decoration-pine'
              >
                {tech.name}
              </a>
            ))}
          </div>
        </div>

        {/* Category 3: Tools */}
        <div className='border-t border-ink/10 pt-6'>
          <div className='mb-4 flex items-center gap-3'>
            <Wrench className='h-4 w-4 text-olive' />
            <h4 className='font-mono text-[11px] font-medium uppercase tracking-[0.16em] text-ink-faint'>Tools</h4>
          </div>
          <div className='flex flex-wrap gap-x-5 gap-y-2.5'>
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
                className='text-sm text-ink-soft underline decoration-ink/20 underline-offset-4 transition-colors hover:text-ink hover:decoration-pine'
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
