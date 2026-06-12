'use client';

import { motion } from 'framer-motion';
import { AnimatedCTATitle } from '../components/AnimatedCTATitle';
import { Spirograph } from '../art/Spirograph';

// Warm oil-paint texture behind the closing band. Ships with the site so the
// colophon never depends on external assets; the veil keeps text readable
// even if the image fails to load.
const OIL_TEXTURE_SRC = '/images/textures/oil-olive-landscape.jpg';

export const RedesignedCTA = () => (
  <section className='selection-paper relative overflow-hidden bg-ink px-6 py-24 md:py-36 text-center text-cream'>
    {/* Oil-paint backdrop under a darkening veil */}
    <div
      aria-hidden='true'
      className='absolute inset-0 bg-cover'
      style={{ backgroundImage: `url('${OIL_TEXTURE_SRC}')`, backgroundPosition: 'center 60%' }}
    />
    <div
      aria-hidden='true'
      className='absolute inset-0'
      style={{
        background:
          'linear-gradient(180deg, rgba(20,19,16,0.66), rgba(20,19,16,0.5) 50%, rgba(20,19,16,0.72))',
      }}
    />

    <div className='relative mx-auto max-w-3xl'>
      <motion.div
        initial={{ opacity: 0 }}
        whileInView={{ opacity: 1 }}
        viewport={{ once: true }}
        transition={{ duration: 0.8 }}
        className='mb-8 flex justify-center'
        aria-hidden='true'
      >
        <Spirograph variant='moire' size={56} stroke='#93A286' strokeWidth={2.6} strokeOpacity={0.7} copies={12} />
      </motion.div>
      <div className='mb-6'>
        <AnimatedCTATitle />
      </div>
      <motion.p
        initial={{ opacity: 0, y: 20 }}
        whileInView={{ opacity: 1, y: 0 }}
        viewport={{ once: true }}
        transition={{ duration: 0.5, delay: 0.1 }}
        className='mb-9 text-base leading-relaxed text-cream/65'
      >
        Build with agents, build for agents, and run it where your data lives.
      </motion.p>
      <motion.div
        initial={{ opacity: 0, y: 20 }}
        whileInView={{ opacity: 1, y: 0 }}
        viewport={{ once: true }}
        transition={{ duration: 0.5, delay: 0.2 }}
        className='flex flex-col items-center justify-center gap-3 sm:flex-row'
      >
        <a
          href='/docs/actors'
          className='inline-flex items-center justify-center whitespace-nowrap rounded-md bg-cream px-4 py-2 text-sm font-medium text-ink transition-colors hover:bg-white'
        >
          Start Building
        </a>
        <a
          href='/talk-to-an-engineer'
          className='inline-flex items-center justify-center whitespace-nowrap rounded-md border border-cream/30 px-4 py-2 text-sm text-cream transition-colors hover:border-cream/60'
        >
          Talk to an Engineer
        </a>
      </motion.div>
    </div>
  </section>
);
