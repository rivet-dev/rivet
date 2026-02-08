'use client';

import { useEffect, useState } from 'react';
import { Database, Cpu, Clock, Wifi, Moon, Shield, Globe } from 'lucide-react';
import { motion } from 'framer-motion';
import actorsLogo from '@/images/products/actors-logo.svg';

const ActorDiagram = () => {
  const [activeStep, setActiveStep] = useState(0);

  useEffect(() => {
    const interval = setInterval(() => {
      setActiveStep(prev => (prev + 1) % 4);
    }, 2000);
    return () => clearInterval(interval);
  }, []);

  return (
    <div className='flex h-full flex-col items-center justify-center p-6'>
      <div className='relative flex items-center gap-6'>
        {/* Client */}
        <div
          className={`rounded border px-3 py-1.5 ${
            activeStep % 2 === 0 ? 'border-white bg-white text-black' : 'border-zinc-700 text-zinc-500'
          } font-mono text-xs transition-colors`}
        >
          Client
        </div>

        {/* Connection line */}
        <div
          className={`h-[1px] w-12 ${
            activeStep % 2 === 0 ? 'bg-white' : 'bg-zinc-800'
          } relative transition-colors`}
        >
          <div
            className={`absolute -top-1 right-0 h-2 w-2 border-r border-t ${
              activeStep % 2 === 0 ? 'border-white' : 'border-zinc-800'
            } rotate-45`}
          />
          <div
            className={`absolute -top-1 left-0 h-2 w-2 border-b border-l ${
              activeStep % 2 === 0 ? 'border-white' : 'border-zinc-800'
            } rotate-45`}
          />
        </div>

        {/* The Actor */}
        <div className='relative'>
          <div
            className={`h-28 w-28 rounded-xl border ${
              activeStep % 2 !== 0
                ? 'border-[#FF4500] bg-[#FF4500]/10'
                : 'border-zinc-700 bg-zinc-900/50'
            } flex flex-col items-center justify-center gap-1.5 transition-all`}
          >
            <div className='font-mono text-xs font-bold text-white'>Actor</div>
            <div className='h-[1px] w-full bg-white/10' />
            <div className='flex items-center gap-1.5'>
              <Cpu className='h-3 w-3 text-zinc-400' />
              <span className='text-[9px] text-zinc-400'>Compute</span>
            </div>
            <div className='flex items-center gap-1.5'>
              <Database className='h-3 w-3 text-[#FF4500]' />
              <span className='text-[9px] text-[#FF4500]'>In-Mem State</span>
            </div>
          </div>
          {/* Pulse effect */}
          {activeStep % 2 !== 0 && (
            <div className='absolute inset-0 animate-ping rounded-xl border border-[#FF4500] opacity-20' />
          )}
        </div>
      </div>

      <p className='mt-4 text-xs text-[#FF4500]'>
        Open source. Runs anywhere.
      </p>
    </div>
  );
};

const FeatureCard = ({ icon: Icon, title, description }: { icon: typeof Database; title: string; description: string }) => (
  <div className='group relative flex h-full flex-col overflow-hidden rounded-xl border border-white/5 bg-zinc-900/30 p-4 backdrop-blur-sm transition-all duration-300 hover:border-white/10'>
    <div className='absolute left-0 right-0 top-0 h-[1px] bg-gradient-to-r from-transparent via-white/20 to-transparent' />
    <div className='pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_top_left,rgba(255,69,0,0.1)_0%,transparent_50%)] opacity-0 transition-opacity duration-500 group-hover:opacity-100' />
    <div className='pointer-events-none absolute left-0 top-0 z-20 h-16 w-16 rounded-tl-xl border-l border-t border-[#FF4500] opacity-0 transition-opacity duration-500 [mask-image:linear-gradient(135deg,black_0%,transparent_50%)] group-hover:opacity-100' />

    <div className='relative z-10 mb-2 rounded bg-[#FF4500]/10 p-2 text-[#FF4500] transition-all duration-500 group-hover:bg-[#FF4500]/20 group-hover:shadow-[0_0_15px_rgba(255,69,0,0.5)] w-fit'>
      <Icon className='h-4 w-4' />
    </div>

    <h3 className='relative z-10 mb-1 text-sm font-medium text-white'>{title}</h3>
    <p className='relative z-10 text-xs leading-relaxed text-zinc-400'>{description}</p>
  </div>
);

const DiagramCard = () => (
  <div className='group relative flex h-full flex-col overflow-hidden rounded-xl border border-[#FF4500]/20 bg-zinc-900/30 backdrop-blur-sm transition-all duration-300 hover:border-[#FF4500]/30 hover:shadow-[0_0_50px_-12px_rgba(255,69,0,0.15)]'>
    <div className='absolute left-0 right-0 top-0 z-10 h-[1px] bg-gradient-to-r from-transparent via-[#FF4500]/40 to-transparent' />
    <div className='pointer-events-none absolute inset-0 bg-gradient-to-b from-[#FF4500]/5 to-transparent' />
    <ActorDiagram />
  </div>
);

export const FeaturesSection = () => {
  const features = [
    {
      icon: Database,
      title: 'In-Memory State',
      description: 'Co-located with compute. Instant reads and writes.'
    },
    {
      icon: Shield,
      title: 'Persistent Storage',
      description: 'Survives restarts, crashes, and deploys.'
    },
    {
      icon: Clock,
      title: 'Workflows',
      description: 'Multi-step operations with automatic retries.'
    },
    {
      icon: Wifi,
      title: 'WebSockets',
      description: 'Real-time streaming built in.'
    },
    {
      icon: Moon,
      title: 'Sleeps When Idle',
      description: 'Wake instantly. Pay only for active users.'
    },
    {
      icon: Globe,
      title: 'Runs Indefinitely',
      description: 'No Lambda timeouts or cron workarounds.'
    }
  ];

  return (
    <section id='features' className='relative bg-black py-20'>
      <div className='mx-auto max-w-7xl px-6'>
        {/* Bento Grid: 4 columns */}
        <div className='grid grid-cols-2 gap-3 lg:grid-cols-4'>
          {/* Row 1: Title (2 cols) + 2 features */}
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            transition={{ duration: 0.5 }}
            className='col-span-2 flex flex-col justify-center p-4'
          >
            <div className='mb-2 flex items-center gap-3'>
              <img src={actorsLogo.src} alt='Rivet Actors' className='h-7' />
              <span className='text-2xl font-medium text-white'>Rivet Actors</span>
            </div>
            <p className='text-sm leading-relaxed text-zinc-400'>
              Long-lived stateful processes with built-in features for the next generation of software.
            </p>
          </motion.div>
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            transition={{ duration: 0.5, delay: 0.05 }}
          >
            <FeatureCard {...features[0]} />
          </motion.div>
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            transition={{ duration: 0.5, delay: 0.1 }}
          >
            <FeatureCard {...features[1]} />
          </motion.div>

          {/* Row 2-3: Diagram (2x2) + 4 features */}
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            transition={{ duration: 0.5, delay: 0.15 }}
            className='col-span-2 row-span-2'
          >
            <DiagramCard />
          </motion.div>
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            transition={{ duration: 0.5, delay: 0.2 }}
          >
            <FeatureCard {...features[2]} />
          </motion.div>
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            transition={{ duration: 0.5, delay: 0.25 }}
          >
            <FeatureCard {...features[3]} />
          </motion.div>
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            transition={{ duration: 0.5, delay: 0.3 }}
          >
            <FeatureCard {...features[4]} />
          </motion.div>
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            transition={{ duration: 0.5, delay: 0.35 }}
          >
            <FeatureCard {...features[5]} />
          </motion.div>
        </div>
      </div>
    </section>
  );
};
