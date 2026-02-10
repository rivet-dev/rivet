'use client';

import { motion } from 'framer-motion';

export function AnimatedCTATitle() {
  return (
    <motion.h2
      initial={{ opacity: 0, y: 20 }}
      whileInView={{ opacity: 1, y: 0 }}
      viewport={{ once: true }}
      transition={{ duration: 0.5 }}
      className='text-2xl font-normal tracking-tight text-white md:text-4xl'
    >
      Infrastructure for software that thinks.
    </motion.h2>
  );
}
