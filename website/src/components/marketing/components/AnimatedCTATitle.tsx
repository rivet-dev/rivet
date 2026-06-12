'use client';

import { motion } from 'framer-motion';

// Renders inside the dark colophon band, so the title is cream.
export function AnimatedCTATitle() {
  return (
    <motion.h2
      initial={{ opacity: 0, y: 20 }}
      whileInView={{ opacity: 1, y: 0 }}
      viewport={{ once: true }}
      transition={{ duration: 0.5 }}
      className='text-3xl font-medium tracking-[-0.015em] text-cream md:text-4xl'
    >
      Infrastructure for <br />
      the agentic era.
    </motion.h2>
  );
}
