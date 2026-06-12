'use client';

import { motion } from 'framer-motion';
import { Lock, Building2, ShieldCheck, ArrowRight } from 'lucide-react';

const points = [
  {
    icon: Lock,
    title: 'Air-gapped and on-prem',
    body: 'Run the same Rivet that powers our cloud entirely inside your perimeter. No outbound calls, no telemetry leaving your boundary.',
  },
  {
    icon: Building2,
    title: 'Embed in your customers',
    body: 'Ship Rivet inside your customer’s VPC, regulated environment, or on-prem hardware. They keep their data, you keep your product.',
  },
  {
    icon: ShieldCheck,
    title: 'Your compliance posture, intact',
    body: 'FedRAMP, HIPAA, regulated industries, sovereign clouds. Deploying inside the boundary your existing controls already cover keeps the audit story simple.',
  },
];

export const OnPremSection = () => (
  <section className='border-t border-white/10 px-6 py-16 md:py-48'>
    <div className='mx-auto max-w-7xl'>
      <div className='mb-12 max-w-3xl'>
        <motion.h2
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          transition={{ duration: 0.5, delay: 0.05 }}
          className='mb-4 text-3xl font-medium tracking-[-0.015em] text-white md:text-4xl'
        >
          Run it where your data lives.
        </motion.h2>
        <motion.p
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          transition={{ duration: 0.5, delay: 0.1 }}
          className='text-base leading-relaxed text-zinc-500 md:text-lg'
        >
          A single binary you control. Deploy Rivet inside your VPC, your customer’s VPC, or fully air-gapped. Use the compliance you already have instead of waiting on someone else’s.
        </motion.p>
      </div>

      <div className='grid grid-cols-1 gap-6 md:grid-cols-3'>
        {points.map((point, idx) => {
          const Icon = point.icon;
          return (
            <motion.div
              key={point.title}
              initial={{ opacity: 0, y: 20 }}
              whileInView={{ opacity: 1, y: 0 }}
              viewport={{ once: true }}
              transition={{ duration: 0.5, delay: 0.05 * idx }}
              className='flex flex-col border-t border-white/10 pt-6'
            >
              <div className='mb-3 text-zinc-500'>
                <Icon className='h-4 w-4' />
              </div>
              <h3 className='mb-2 text-base font-medium text-white'>{point.title}</h3>
              <p className='text-sm leading-relaxed text-zinc-500'>{point.body}</p>
            </motion.div>
          );
        })}
      </div>

      <motion.div
        initial={{ opacity: 0, y: 20 }}
        whileInView={{ opacity: 1, y: 0 }}
        viewport={{ once: true }}
        transition={{ duration: 0.5, delay: 0.2 }}
        className='mt-12 flex flex-col items-center justify-center gap-3 sm:flex-row'
      >
        <a
          href='/talk-to-an-engineer'
          className='inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md bg-white px-4 py-2 text-sm font-medium text-black transition-colors hover:bg-zinc-200'
        >
          Talk to an engineer
          <ArrowRight className='h-4 w-4' />
        </a>
        <a
          href='/docs/general/self-hosting'
          className='inline-flex items-center justify-center whitespace-nowrap rounded-md border border-white/10 px-4 py-2 text-sm text-zinc-300 transition-colors hover:border-white/20 hover:text-white'
        >
          Read self-hosting docs
        </a>
      </motion.div>
    </div>
  </section>
);
