'use client';

import { motion } from 'framer-motion';

const StatItem = ({ value, label }) => (
  <div className='group flex flex-col items-start border-l border-white/10 pl-6 transition-colors duration-500 hover:border-[#FF4500]/50'>
    <span className='mb-1 text-3xl font-medium tracking-tighter text-white transition-colors group-hover:text-[#FF4500]'>
      {value}
    </span>
    <span className='text-xs font-medium uppercase tracking-widest text-zinc-500'>{label}</span>
  </div>
);

export const StatsSection = () => (
  <section className='border-y border-white/5 bg-white/[0.02] backdrop-blur-sm'>
    <div className='mx-auto max-w-7xl px-6 py-16'>
      <motion.div
        initial={{ opacity: 0, y: 20 }}
        whileInView={{ opacity: 1, y: 0 }}
        viewport={{ once: true }}
        transition={{ duration: 0.5, staggerChildren: 0.1 }}
        className='grid grid-cols-2 gap-8 md:grid-cols-4 md:gap-0'
      >
        <StatItem value='< 1ms' label='Read/Write Latency' />
        <StatItem value='∞' label='Horizontal Scale' />
        <StatItem value='100%' label='Self-Hostable' />
        <StatItem value='Apache 2.0' label='Permissive Open-Source' />
      </motion.div>
    </div>
  </section>
);
