'use client';

import { useEffect } from 'react';

const problems = [
  {
    category: 'Problem',
    number: '01',
    icon: (
      <svg
        xmlns='http://www.w3.org/2000/svg'
        className='h-6 w-6 text-red-500'
        viewBox='0 0 24 24'
        fill='none'
        stroke='currentColor'
        strokeWidth='1.5'
        strokeLinecap='round'
        strokeLinejoin='round'
      >
        <circle cx='12' cy='12' r='10'></circle>
        <line x1='12' y1='8' x2='12' y2='12'></line>
        <line x1='12' y1='16' x2='12.01' y2='16'></line>
      </svg>
    ),
    title: 'The Old Way',
    description: 'Your serverless function spins up. It queries a database. It does its job and dies.',
    code: 'lambda()',
    status: 'The pain',
    statusColor: 'bg-red-500'
  },
  {
    category: 'Complexity',
    number: '02',
    icon: (
      <svg
        xmlns='http://www.w3.org/2000/svg'
        className='h-6 w-6 text-yellow-500'
        viewBox='0 0 24 24'
        fill='none'
        stroke='currentColor'
        strokeWidth='1.5'
        strokeLinecap='round'
        strokeLinejoin='round'
      >
        <path d='M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z'></path>
        <polyline points='3.27 6.96 12 12.01 20.73 6.96'></polyline>
        <line x1='12' y1='22.08' x2='12' y2='12'></line>
      </svg>
    ),
    title: 'The "Glue Code" Mess',
    description:
      'So you add Redis for state. And a message queue for jobs. And a WebSocket server for realtime. Your app is now a complex, distributed monolith.',
    code: 'redis + queue + ws',
    status: 'Complexity',
    statusColor: 'bg-yellow-500'
  },
  {
    category: 'Solution',
    number: '03',
    icon: (
      <svg
        xmlns='http://www.w3.org/2000/svg'
        className='h-6 w-6 text-green-500'
        viewBox='0 0 24 24'
        fill='none'
        stroke='currentColor'
        strokeWidth='1.5'
        strokeLinecap='round'
        strokeLinejoin='round'
      >
        <path d='M22 11.08V12a10 10 0 1 1-5.93-9.14'></path>
        <polyline points='22 4 12 14.01 9 11.01'></polyline>
      </svg>
    ),
    title: 'The Rivet Way',
    description:
      "Your logic and state live together. One library. One process. It's fast, resilient, and scales from zero. No database round-trips. No queues. Just code.",
    code: 'actor()',
    status: 'The gain',
    statusColor: 'bg-green-500'
  }
];

export function ProblemSection() {
  useEffect(() => {
    // Bento box glow effect
    const bentoBoxes = document.querySelectorAll<HTMLDivElement>('.bento-box');
    bentoBoxes.forEach(box => {
      const handleMouseMove = (e: MouseEvent) => {
        const rect = box.getBoundingClientRect();
        const x = e.clientX - rect.left;
        const y = e.clientY - rect.top;
        box.style.setProperty('--mouse-x', `${x}px`);
        box.style.setProperty('--mouse-y', `${y}px`);
      };

      const handleMouseLeave = () => {
        box.style.setProperty('--mouse-x', '50%');
        box.style.setProperty('--mouse-y', '50%');
      };

      box.addEventListener('mousemove', handleMouseMove);
      box.addEventListener('mouseleave', handleMouseLeave);

      return () => {
        box.removeEventListener('mousemove', handleMouseMove);
        box.removeEventListener('mouseleave', handleMouseLeave);
      };
    });
  }, []);

  return (
    <section className='py-24 md:py-32'>
      <div className='mx-auto max-w-6xl px-4 sm:px-6 lg:px-8'>
        <h2 className='animate-on-scroll animate-fade-up mb-16 text-center font-heading text-4xl font-bold tracking-tighter text-text-primary md:text-5xl'>
          Your Stack is Stateless.
          <br />
          Your Apps Aren't.
        </h2>

        <div className='grid grid-cols-1 gap-6 md:grid-cols-3'>
          {problems.map((problem, index) => {
            const delayClasses = ['delay-100', 'delay-200', 'delay-300'];
            const isSolution = problem.category === 'Solution';
            const isComplexity = problem.category === 'Complexity';
            const isProblem = problem.category === 'Problem';
            return (
              <div
                key={index}
                className={`bento-box animate-on-scroll animate-fade-up ${
                  delayClasses[index] || ''
                } flex flex-col justify-between rounded-xl border border-border bg-background/50 p-6 md:p-8 ${
                  isSolution
                    ? 'ring-1 ring-green-500/30'
                    : isComplexity
                    ? 'ring-1 ring-yellow-500/30'
                    : isProblem
                    ? 'ring-1 ring-red-500/30'
                    : ''
                }`}
              >
                {/* Header with category badge and number */}
                <div className='mb-6 flex items-start justify-between gap-4'>
                  <div
                    className={`inline-flex items-center justify-center rounded-full border border-border bg-black/50 px-3 py-1.5 ${
                      isSolution
                        ? 'border-green-500/30'
                        : isComplexity
                        ? 'border-yellow-500/30'
                        : isProblem
                        ? 'border-red-500/30'
                        : ''
                    }`}
                  >
                    <span
                      className={`h-1.5 w-1.5 rounded-full ${
                        isSolution ? 'bg-green-500' : isComplexity ? 'bg-yellow-500' : 'bg-red-500'
                      } mr-2`}
                    ></span>
                    <span className='text-xs uppercase tracking-wider text-text-secondary'>
                      {problem.category}
                    </span>
                  </div>
                  <span className='font-mono text-xs text-text-secondary'>{problem.number}</span>
                </div>

                {/* Icon, title, and description */}
                <div className='flex-1'>
                  <div className='mb-4 inline-flex items-center justify-center rounded-lg border border-border bg-black/30 p-3'>
                    {problem.icon}
                  </div>
                  <h3
                    className={`mb-2 font-heading text-xl font-bold ${
                      isSolution ? 'text-green-500' : isComplexity ? 'text-yellow-500' : 'text-text-primary'
                    }`}
                  >
                    {problem.title}
                  </h3>
                  <p className='text-text-secondary'>{problem.description}</p>
                </div>
              </div>
            );
          })}
        </div>
      </div>
    </section>
  );
}
