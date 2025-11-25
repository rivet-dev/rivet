'use client';

import { useRef, useState } from 'react';
import { useCases } from '@/data/use-cases';
import Link from 'next/link';
import { IconWithSpotlight } from '../sections/IconWithSpotlight';
import { AnimatePresence, motion } from 'framer-motion';

interface UseCaseCardProps {
  title: string;
  description: React.ReactNode;
  href: string;
  className?: string;
  iconPath: string;
  variant?: 'default' | 'large';
  onHover?: (title: string | null) => void;
}

function UseCaseCard({
  title,
  description,
  href,
  className = '',
  iconPath,
  variant = 'default',
  onHover
}: UseCaseCardProps) {
  const cardRef = useRef<HTMLAnchorElement>(null);

  const handleMouseMove = (e: React.MouseEvent<HTMLAnchorElement>) => {
    if (!cardRef.current) return;
    const card = cardRef.current;

    // Find the icon container and convert coordinates to icon-relative
    const iconContainer = card.querySelector('.icon-spotlight-container') as HTMLElement;
    if (!iconContainer) return;

    // Get the icon's position relative to viewport
    const iconRect = iconContainer.getBoundingClientRect();

    // Calculate mouse position relative to the icon (not the card)
    const x = ((e.clientX - iconRect.left) / iconRect.width) * 100;
    const y = ((e.clientY - iconRect.top) / iconRect.height) * 100;

    // Set CSS custom properties on the icon container
    iconContainer.style.setProperty('--mouse-x', `${x}%`);
    iconContainer.style.setProperty('--mouse-y', `${y}%`);
  };

  if (variant === 'large') {
    return (
      <Link
        ref={cardRef}
        href={href}
        className={`group relative block ${className}`}
        onMouseMove={handleMouseMove}
        onMouseEnter={() => onHover?.(title)}
        onMouseLeave={() => onHover?.(null)}
      >
        <div className='relative flex h-full flex-row gap-6 overflow-hidden rounded-xl border border-white/20 bg-white/[0.008] p-6 transition-colors hover:border-white/30 hover:bg-white/[0.02]'>
          {/* Gradient overlay on hover */}
          <div className='pointer-events-none absolute inset-0 bg-gradient-to-b from-white/[0.04] via-white/[0.01] to-transparent opacity-0 transition-opacity duration-300 group-hover:opacity-100' />

          {/* Left side: Content + Checkmarks */}
          <div className='relative z-10 flex flex-1 flex-col justify-between'>
            <div>
              <h3 className='mb-3 text-lg font-semibold text-white'>{title}</h3>
              <p className='text-sm leading-relaxed text-white/40'>{description}</p>
            </div>

            {/* Checkmarks */}
            <div className='mt-6 space-y-2'>
              {['Cloud & on-prem', 'Supports realtime', 'Works with AI SDK'].map(item => (
                <div key={item} className='flex items-center gap-2'>
                  <svg
                    className='h-4 w-4 text-white/60'
                    fill='none'
                    stroke='currentColor'
                    viewBox='0 0 24 24'
                  >
                    <path strokeLinecap='round' strokeLinejoin='round' strokeWidth={2} d='M5 13l4 4L19 7' />
                  </svg>
                  <span className='text-sm text-white/60'>{item}</span>
                </div>
              ))}
            </div>
          </div>

          {/* Right side: Icon */}
          <div className='relative z-10 flex flex-1 items-center justify-center'>
            <IconWithSpotlight iconPath={iconPath} title={title} />
          </div>
        </div>
      </Link>
    );
  }

  return (
    <Link
      ref={cardRef}
      href={href}
      className={`group relative block ${className}`}
      onMouseMove={handleMouseMove}
      onMouseEnter={() => onHover?.(title)}
      onMouseLeave={() => onHover?.(null)}
    >
      <div className='relative flex h-full flex-col overflow-hidden rounded-xl border border-white/20 bg-white/[0.008] p-6 transition-colors hover:border-white/30 hover:bg-white/[0.02]'>
        {/* Gradient overlay on hover */}
        <div className='pointer-events-none absolute inset-0 bg-gradient-to-b from-white/[0.04] via-white/[0.01] to-transparent opacity-0 transition-opacity duration-300 group-hover:opacity-100' />

        {/* Content */}
        <div className='relative z-10 mb-6'>
          <h3 className='mb-3 text-lg font-semibold text-white'>{title}</h3>
          <p className='text-sm leading-relaxed text-white/40'>{description}</p>
        </div>

        {/* Spotlight icon */}
        <div className='relative z-10 flex flex-1 items-center justify-center'>
          <IconWithSpotlight iconPath={iconPath} title={title} />
        </div>
      </div>
    </Link>
  );
}

export function UseCases() {
  const [hoveredTitle, setHoveredTitle] = useState<string | null>(null);

  // Map the use cases we want to display
  const selectedUseCases = [
    useCases.find(uc => uc.title === 'Agent Orchestration & MCP')!, // agent orchestration & mcp
    useCases.find(uc => uc.title === 'Workflows')!, // workflows
    useCases.find(uc => uc.title === 'Multiplayer Apps')!, // multiplayer apps
    useCases.find(uc => uc.title === 'Local-First Sync')!, // local-first sync
    useCases.find(uc => uc.title === 'Background Jobs')!, // background jobs
    useCases.find(uc => uc.title === 'Per-Tenant Databases')!, // per-tenant databases
    useCases.find(uc => uc.title === 'Geo-Distributed Database')! // geo-distributed database
  ];

  // Map use case titles to icon paths
  const getIconPath = (title: string): string => {
    const iconMap: { [key: string]: string } = {
      'Agent Orchestration & MCP': '/use-case-icons/sparkles.svg',
      Workflows: '/use-case-icons/diagram-next.svg',
      'Multiplayer Apps': '/use-case-icons/file-pen.svg',
      'Local-First Sync': '/use-case-icons/rotate.svg',
      'Background Jobs': '/use-case-icons/gears.svg',
      'Per-Tenant Databases': '/use-case-icons/database.svg',
      'Geo-Distributed Database': '/use-case-icons/globe.svg'
    };
    return iconMap[title] || '';
  };

  // Get highlighted description
  const getHighlightedDescription = (title: string): React.ReactNode => {
    const descriptionMap: { [key: string]: React.ReactNode } = {
      'Agent Orchestration & MCP': (
        <>
          Build <span className='text-white/90'>AI agents</span> with Model Context Protocol and persistent
          state
        </>
      ),
      Workflows: (
        <>
          <span className='text-white/90'>Durable multi-step workflows</span> with automatic state management
        </>
      ),
      'Multiplayer Apps': (
        <>
          Build <span className='text-white/90'>realtime multiplayer</span> applications with authoritative
          state
        </>
      ),
      'Local-First Sync': (
        <>
          <span className='text-white/90'>Offline-first</span> applications with server synchronization
        </>
      ),
      'Background Jobs': (
        <>
          <span className='text-white/90'>Scheduled and recurring jobs</span> without external queue
          infrastructure
        </>
      ),
      'Per-Tenant Databases': (
        <>
          <span className='text-white/90'>Isolated data stores</span> for each user with zero-latency access
        </>
      ),
      'Geo-Distributed Database': (
        <>
          Store data close to users globally with{' '}
          <span className='text-white/90'>automatic edge distribution</span>
        </>
      )
    };
    return descriptionMap[title] || '';
  };

  return (
    <section className='w-full'>
      <div className='container relative mx-auto max-w-[1500px] px-6 lg:px-16 xl:px-20'>
        <h2 className='font-700 mb-8 text-left text-2xl text-white sm:text-3xl'>
          Actors make it simple to build{' '}
          <span className='relative inline-block'>
            <AnimatePresence mode='wait'>
              {hoveredTitle && (
                <motion.span
                  key={hoveredTitle}
                  initial={{ opacity: 0, y: 20 }}
                  animate={{ opacity: 1, y: 0 }}
                  exit={{ opacity: 0, y: -10 }}
                  transition={{ duration: 0.15, ease: 'easeOut' }}
                  className='inline-block text-[#FF5C00]'
                >
                  {hoveredTitle}
                </motion.span>
              )}
            </AnimatePresence>
          </span>
        </h2>
        <div className='grid auto-rows-[300px] grid-cols-1 gap-4 sm:grid-cols-2 md:grid-cols-12 md:gap-4 xl:gap-3 2xl:gap-6'>
          {/* First item - takes 6 columns (half width on medium+) */}
          <UseCaseCard
            title={selectedUseCases[0].title}
            description={getHighlightedDescription(selectedUseCases[0].title)}
            href={selectedUseCases[0].href}
            iconPath={getIconPath(selectedUseCases[0].title)}
            className='sm:col-span-2 md:col-span-6'
            variant='large'
            onHover={setHoveredTitle}
          />

          {/* Remaining items - 3 columns each (quarter width on medium+) */}
          {selectedUseCases.slice(1).map(useCase => (
            <UseCaseCard
              key={useCase.href}
              title={useCase.title}
              description={getHighlightedDescription(useCase.title)}
              href={useCase.href}
              iconPath={getIconPath(useCase.title)}
              className='md:col-span-3'
              onHover={setHoveredTitle}
            />
          ))}
        </div>
      </div>
    </section>
  );
}
