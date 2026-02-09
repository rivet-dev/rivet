'use client';

import { motion } from 'framer-motion';
import { AnimatedCTATitle } from '../components/AnimatedCTATitle';

export const RedesignedCTA = () => (
  <section className='border-t border-white/10 px-6 py-48 text-center'>
    <div className='mx-auto max-w-3xl'>
      <div className='mb-6'>
        <AnimatedCTATitle />
      </div>
      <motion.p
        initial={{ opacity: 0, y: 20 }}
        whileInView={{ opacity: 1, y: 0 }}
        viewport={{ once: true }}
        transition={{ duration: 0.5, delay: 0.1 }}
        className='mb-8 text-base leading-relaxed text-zinc-500'
      >
        The next generation of software needs a new kind of backend. This is it.
      </motion.p>
      <motion.div
        initial={{ opacity: 0, y: 20 }}
        whileInView={{ opacity: 1, y: 0 }}
        viewport={{ once: true }}
        transition={{ duration: 0.5, delay: 0.2 }}
        className='flex flex-col items-center justify-center gap-3 sm:flex-row'
      >
        <a
          href='/docs'
          className='inline-flex items-center justify-center whitespace-nowrap rounded-md bg-white px-4 py-2 text-sm font-medium text-black transition-colors hover:bg-zinc-200'
        >
          Start Building
        </a>
        <a
          href='/talk-to-an-engineer'
          className='inline-flex items-center justify-center whitespace-nowrap rounded-md border border-white/10 px-4 py-2 text-sm text-zinc-300 transition-colors hover:border-white/20 hover:text-white'
        >
          Talk to an Engineer
        </a>
      </motion.div>
    </div>
  </section>
);
