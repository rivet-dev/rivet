'use client';

import { useEffect } from 'react';

const primitives = [
  {
    category: 'Compute',
    number: '01',
    icon: (
      <svg
        xmlns='http://www.w3.org/2000/svg'
        className='h-6 w-6 text-accent'
        viewBox='0 0 24 24'
        fill='none'
        stroke='currentColor'
        strokeWidth='1.5'
        strokeLinecap='round'
        strokeLinejoin='round'
      >
        <rect x='4' y='4' width='16' height='16' rx='2'></rect>
        <rect x='9' y='9' width='6' height='6'></rect>
        <path d='M9 1v3'></path>
        <path d='M15 1v3'></path>
        <path d='M9 20v3'></path>
        <path d='M15 20v3'></path>
        <path d='M20 9h3'></path>
        <path d='M20 14h3'></path>
        <path d='M1 9h3'></path>
        <path d='M1 14h3'></path>
      </svg>
    ),
    title: 'Long-Lived Compute',
    description:
      'Like Lambda, but with memory. No 5-minute timeouts, no state loss. Your logic runs as long as your product does.',
    code: 'persistent.process()'
  },
  {
    category: 'State',
    number: '02',
    icon: (
      <svg
        xmlns='http://www.w3.org/2000/svg'
        className='h-6 w-6 text-accent'
        viewBox='0 0 24 24'
        fill='none'
        stroke='currentColor'
        strokeWidth='1.5'
        strokeLinecap='round'
        strokeLinejoin='round'
      >
        <path d='M12 14l4-4'></path>
        <path d='M3.34 19a10 10 0 1 1 17.32 0'></path>
      </svg>
    ),
    title: 'Zero-Latency State',
    description:
      'State lives beside your compute. Reads and writes are in-memory—no cache invalidation, no round-trips, no gymnastics.',
    code: 'read(<1ms)'
  },
  {
    category: 'Realtime',
    number: '03',
    icon: (
      <svg
        xmlns='http://www.w3.org/2000/svg'
        className='h-6 w-6 text-accent'
        viewBox='0 0 24 24'
        fill='none'
        stroke='currentColor'
        strokeWidth='1.5'
        strokeLinecap='round'
        strokeLinejoin='round'
      >
        <circle cx='12' cy='12' r='1'></circle>
        <path d='M5 8.55a7 7 0 0 1 0 6.9'></path>
        <path d='M19 8.55a7 7 0 0 0 0 6.9'></path>
        <path d='M8.5 6a10 10 0 0 1 0 12'></path>
        <path d='M15.5 6a10 10 0 0 0 0 12'></path>
      </svg>
    ),
    title: 'Realtime, Built-in',
    description:
      'WebSockets and SSE out of the box. Broadcast updates with one line—no extra infrastructure, no pub/sub layer.',
    code: 'c.broadcast()'
  },
  {
    category: 'Economics',
    number: '04',
    icon: (
      <svg
        xmlns='http://www.w3.org/2000/svg'
        className='h-6 w-6 text-accent'
        viewBox='0 0 24 24'
        fill='none'
        stroke='currentColor'
        strokeWidth='1.5'
        strokeLinecap='round'
        strokeLinejoin='round'
      >
        <path d='M12 3a6 6 0 0 0 9 9 9 9 0 1 1-9-9z'></path>
      </svg>
    ),
    title: 'Sleep When Idle',
    description:
      'Actors automatically hibernate to save costs and wake instantly on demand. You only pay for work done—not uptime.',
    code: 'idle → sleep()'
  },
  {
    category: 'Ownership',
    number: '05',
    icon: (
      <svg
        xmlns='http://www.w3.org/2000/svg'
        className='h-6 w-6 text-accent'
        viewBox='0 0 24 24'
        fill='none'
        stroke='currentColor'
        strokeWidth='1.5'
        strokeLinecap='round'
        strokeLinejoin='round'
      >
        <path d='M15 22v-2.5a3.37 3.37 0 0 0-.94-2.61c3.14-.35 6.44-1.54 6.44-7A5.44 5.44 0 0 0 19.5 5.77 5.07 5.07 0 0 0 19.4 2S18.27 1.65 15 3.46a13.38 13.38 0 0 0-6 0C5.73 1.65 4.6 2 4.6 2a5.07 5.07 0 0 0-.1 3.77A5.44 5.44 0 0 0 3.5 9.89c0 5.42 3.3 6.61 6.44 7A3.37 3.37 0 0 0 9 19.5V22'></path>
        <path d='M9 18c-4.51 2-5-2-7-2'></path>
      </svg>
    ),
    title: 'Open Source & Self-Hostable',
    description:
      'No lock-in. Run on your platform of choice or bare metal with the same API and mental model.',
    code: 'apache-2.0'
  },
  {
    category: 'Reliability',
    number: '06',
    icon: (
      <svg
        xmlns='http://www.w3.org/2000/svg'
        className='h-6 w-6 text-accent'
        viewBox='0 0 24 24'
        fill='none'
        stroke='currentColor'
        strokeWidth='1.5'
        strokeLinecap='round'
        strokeLinejoin='round'
      >
        <path d='m9 12 2 2 4-4'></path>
        <path d='M12 22c4.5-2 8-5 8-10V5l-8-3-8 3v7c0 5 3.5 8 8 10'></path>
      </svg>
    ),
    title: 'Resilient by Design',
    description:
      'Automatic failover and restarts maintain state integrity. Your actors survive crashes, deploys, noisy neighbors.',
    code: 'uptime > 99.9%'
  }
];

export function NewFeaturesBento() {
  useEffect(() => {
    // Scroll animation setup using existing system
    const scrollElements = document.querySelectorAll('.animate-on-scroll');

    const observer = new IntersectionObserver(
      entries => {
        entries.forEach(entry => {
          if (entry.isIntersecting) {
            entry.target.classList.add('is-visible');
            observer.unobserve(entry.target);
          }
        });
      },
      { threshold: 0.1 }
    );

    scrollElements.forEach(el => observer.observe(el));

    return () => {
      scrollElements.forEach(el => observer.unobserve(el));
    };
  }, []);

  return (
    <section className='py-24 md:py-32'>
      <div className='mx-auto max-w-6xl px-4 sm:px-6 lg:px-8'>
        <h2 className='animate-on-scroll animate-fade-up mb-16 text-center font-heading text-4xl font-bold tracking-tighter text-text-primary md:text-5xl'>
          A New Primitive for Your Backend.
        </h2>

        <div className='grid grid-cols-1 gap-6 md:grid-cols-2 lg:grid-cols-3'>
          {primitives.map((primitive, index) => {
            const delayClasses = [
              'delay-100',
              'delay-200',
              'delay-300',
              'delay-400',
              'delay-500',
              'delay-600'
            ];
            return (
              <div
                key={index}
                className={`bento-box animate-on-scroll animate-fade-up ${
                  delayClasses[index] || ''
                } flex flex-col justify-between rounded-xl border border-border bg-background/50 p-6 md:p-8`}
              >
                {/* Header with category badge and number */}
                <div className='mb-6 flex items-start justify-between gap-4'>
                  <div className='inline-flex items-center justify-center rounded-full border border-border bg-black/50 px-3 py-1.5'>
                    <span className='mr-2 h-1.5 w-1.5 rounded-full bg-accent'></span>
                    <span className='text-xs uppercase tracking-wider text-text-secondary'>
                      {primitive.category}
                    </span>
                  </div>
                  <span className='font-mono text-xs text-text-secondary'>{primitive.number}</span>
                </div>

                {/* Icon, title, and description */}
                <div className='flex-1'>
                  <div className='mb-4 inline-flex items-center justify-center rounded-lg border border-border bg-black/30 p-3'>
                    {primitive.icon}
                  </div>
                  <h3 className='mb-2 font-heading text-xl font-bold text-text-primary'>{primitive.title}</h3>
                  <p className='text-text-secondary'>{primitive.description}</p>
                </div>

                {/* Footer with code */}
                <div className='mt-6 border-t border-border pt-4'>
                  <span className='font-mono text-xs text-text-secondary'>{primitive.code}</span>
                </div>
              </div>
            );
          })}
        </div>
      </div>
    </section>
  );
}
