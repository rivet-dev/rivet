'use client';

import { motion } from 'framer-motion';
import { AnimatedCTATitle } from '../components/AnimatedCTATitle';

export const RedesignedCTA = () => (
  <section className='relative overflow-hidden border-t border-white/10 px-6 py-32 text-center'>
    <div className='absolute inset-0 z-0 bg-gradient-to-b from-black to-zinc-900/50' />
    <motion.div
      animate={{ opacity: [0.3, 0.5, 0.3] }}
      transition={{ duration: 4, repeat: Infinity }}
      className='pointer-events-none absolute inset-0 bg-[radial-gradient(ellipse_at_center,_var(--tw-gradient-stops))] from-[#FF4500]/10 via-transparent to-transparent opacity-50'
    />
    <div className='relative z-10 mx-auto max-w-3xl'>
      <div className='mb-8'>
        <AnimatedCTATitle />
      </div>
      <motion.p
        initial={{ opacity: 0, y: 20 }}
        whileInView={{ opacity: 1, y: 0 }}
        viewport={{ once: true }}
        transition={{ duration: 0.5, delay: 0.1 }}
        className='mb-10 text-lg leading-relaxed text-zinc-400'
      >
        The next generation of software needs a new kind of backend. This is it.
      </motion.p>
      <motion.div
        initial={{ opacity: 0, y: 20 }}
        whileInView={{ opacity: 1, y: 0 }}
        viewport={{ once: true }}
        transition={{ duration: 0.5, delay: 0.2 }}
        className='flex flex-col items-center justify-center gap-4 sm:flex-row'
      >
        <a
          href='/docs'
          className='font-v2 inline-flex items-center justify-center whitespace-nowrap rounded-md border border-white/10 bg-white px-4 py-2 text-sm text-black subpixel-antialiased shadow-sm transition-colors hover:bg-zinc-200'
        >
          Start Building
        </a>
        <a
          href='/talk-to-an-engineer'
          className='font-v2 inline-flex items-center justify-center whitespace-nowrap rounded-md border border-white/10 bg-white/5 px-4 py-2 text-sm text-white subpixel-antialiased shadow-sm transition-colors hover:border-white/20'
        >
          Talk to an Engineer
        </a>
      </motion.div>
    </div>
  </section>
);
