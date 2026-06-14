'use client';

import { motion } from 'framer-motion';
import { AnimatedCTATitle } from '../components/AnimatedCTATitle';

const FOOTER_PAINTING_SRC =
  'https://assets.rivet.dev/website/images/textures/footer-abstract.webp';

export const RedesignedCTA = () => (
  <section className='selection-paper bg-paper px-4 py-14 text-center text-cream md:px-6 md:py-24'>
    <div className='relative mx-auto flex min-h-[26rem] max-w-screen-2xl items-center justify-center overflow-hidden px-6 py-20 md:min-h-[34rem] md:px-10 lg:aspect-[2563/1440] lg:min-h-0'>
      <img
        aria-hidden='true'
        src={FOOTER_PAINTING_SRC}
        alt=''
        loading='eager'
        decoding='async'
        className='absolute inset-0 h-full w-full object-cover object-center lg:object-contain'
      />

      <div className='relative z-[1] mx-auto max-w-3xl'>
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
            className='inline-flex w-full items-center justify-center whitespace-nowrap rounded-md sm:w-auto bg-white px-4 py-2 text-sm font-medium text-ink transition-colors hover:bg-white/90'
          >
            Start Building
          </a>
          <a
            href='/talk-to-an-engineer'
            className='inline-flex w-full items-center justify-center whitespace-nowrap rounded-md sm:w-auto border border-cream/25 bg-cream/10 px-4 py-2 text-sm text-cream backdrop-blur-md backdrop-saturate-[1.3] transition-colors hover:border-cream/45 hover:bg-cream/20'
          >
            Talk to an Engineer
          </a>
        </motion.div>
      </div>
    </div>
  </section>
);
