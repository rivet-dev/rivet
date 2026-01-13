'use client';

import { useRef } from 'react';
import { Icon } from '@rivet-gg/icons';
import {
  faDatabase,
  faBolt,
  faMoon,
  faClock,
  faShieldHalved,
  faRocket,
  faCheckCircle
} from '@rivet-gg/icons';

interface FeatureCardProps {
  title: string;
  description: string;
  href: string;
  className?: string;
  icon: any;
  variant?: 'default' | 'large' | 'medium' | 'small' | 'wide' | 'code';
}

function FeatureCard({
  title,
  description,
  href,
  className = '',
  icon,
  variant = 'default'
}: FeatureCardProps) {
  const cardRef = useRef<HTMLAnchorElement>(null);

  const handleMouseMove = (e: React.MouseEvent<HTMLAnchorElement>) => {
    if (!cardRef.current) return;
    const card = cardRef.current;

    const iconContainer = card.querySelector('.icon-spotlight-container') as HTMLElement;
    if (!iconContainer) return;

    const iconRect = iconContainer.getBoundingClientRect();
    const x = ((e.clientX - iconRect.left) / iconRect.width) * 100;
    const y = ((e.clientY - iconRect.top) / iconRect.height) * 100;

    iconContainer.style.setProperty('--mouse-x', `${x}%`);
    iconContainer.style.setProperty('--mouse-y', `${y}%`);
  };

  if (variant === 'large') {
    return (
      <a
        ref={cardRef}
        href={href}
        className={`group relative block ${className}`}
        onMouseMove={handleMouseMove}
      >
        <div className='relative flex h-full flex-col overflow-hidden rounded-xl border border-white/20 bg-white/[0.008] p-6 transition-all hover:border-white/30 hover:bg-white/[0.02]'>
          <div className='pointer-events-none absolute inset-0 bg-gradient-to-b from-white/[0.04] via-white/[0.01] to-transparent opacity-0 transition-opacity duration-300 group-hover:opacity-100' />

          <div className='relative z-10 flex flex-1 flex-col justify-between'>
            <div>
              <div className='mb-4'>
                <Icon icon={icon} className='h-12 w-12 text-white/70' />
              </div>
              <h3 className='mb-3 text-2xl font-semibold text-white'>{title}</h3>
              <p className='text-base leading-relaxed text-white/50'>{description}</p>
            </div>
          </div>
        </div>
      </a>
    );
  }

  if (variant === 'medium') {
    return (
      <a
        ref={cardRef}
        href={href}
        className={`group relative block ${className}`}
        onMouseMove={handleMouseMove}
      >
        <div className='relative flex h-full flex-col overflow-hidden rounded-xl border border-white/20 bg-white/[0.008] p-6 transition-colors hover:border-white/30 hover:bg-white/[0.02]'>
          <div className='pointer-events-none absolute inset-0 bg-gradient-to-b from-white/[0.04] via-white/[0.01] to-transparent opacity-0 transition-opacity duration-300 group-hover:opacity-100' />

          <div className='relative z-10 mb-4'>
            <Icon icon={icon} className='mb-3 h-10 w-10 text-white/70' />
            <h3 className='mb-2 text-lg font-semibold text-white'>{title}</h3>
            <p className='text-sm leading-relaxed text-white/50'>{description}</p>
          </div>
        </div>
      </a>
    );
  }

  if (variant === 'small') {
    return (
      <a
        ref={cardRef}
        href={href}
        className={`group relative block ${className}`}
        onMouseMove={handleMouseMove}
      >
        <div className='relative flex h-full flex-col overflow-hidden rounded-xl border border-white/20 bg-white/[0.008] p-5 transition-colors hover:border-white/30 hover:bg-white/[0.02]'>
          <div className='pointer-events-none absolute inset-0 bg-gradient-to-b from-white/[0.04] via-white/[0.01] to-transparent opacity-0 transition-opacity duration-300 group-hover:opacity-100' />

          <div className='relative z-10'>
            <div className='mb-3'>
              <Icon icon={icon} className='h-8 w-8 text-white/60' />
            </div>
            <h3 className='mb-2 text-base font-semibold text-white'>{title}</h3>
            <p className='text-sm leading-relaxed text-white/40'>{description}</p>
          </div>
        </div>
      </a>
    );
  }

  if (variant === 'code') {
    return (
      <div className={`group relative block ${className}`}>
        <div className='relative h-full overflow-hidden rounded-xl border border-white/10 bg-[#1e1e1e] p-6 transition-colors hover:border-white/20'>
          <h3 className='mb-4 text-lg font-semibold text-white'>{title}</h3>
          <pre className='overflow-x-auto rounded-lg bg-[#282828] p-4'>
            <code className='whitespace-pre font-mono text-sm leading-relaxed text-[#d4d4d4]'>
              <span className='text-[#c586c0]'>const</span> <span className='text-[#9cdcfe]'>counter</span> ={' '}
              <span className='text-[#dcdcaa]'>actor</span>({`{`}
              {'\n'}
              {`  `}
              <span className='text-[#9cdcfe]'>state</span>: {`{ `}
              <span className='text-[#9cdcfe]'>count</span>: <span className='text-[#b5cea8]'>0</span> {`}`},
              {`\n`}
              {`  `}
              <span className='text-[#9cdcfe]'>actions</span>: {`{`}
              {'\n'}
              {`    `}
              <span className='text-[#dcdcaa]'>increment</span>: (<span className='text-[#9cdcfe]'>c</span>)
              =&gt; {`{`}
              {'\n'}
              {`      `}
              <span className='text-[#9cdcfe]'>c</span>.<span className='text-[#9cdcfe]'>state</span>.
              <span className='text-[#9cdcfe]'>count</span>++;{`\n`}
              {`      `}
              <span className='text-[#9cdcfe]'>c</span>.<span className='text-[#dcdcaa]'>broadcast</span>(
              <span className='text-[#ce9178]'>"changed"</span>, <span className='text-[#9cdcfe]'>c</span>.
              <span className='text-[#9cdcfe]'>state</span>.<span className='text-[#9cdcfe]'>count</span>);
              {`\n`}
              {`    `}
              {`}`}
              {`\n`}
              {`  `}
              {`}`}
              {`\n`}
              {`});`}
            </code>
          </pre>
        </div>
      </div>
    );
  }

  return (
    <a
      ref={cardRef}
      href={href}
      className={`group relative block ${className}`}
      onMouseMove={handleMouseMove}
    >
      <div className='relative flex h-full flex-col overflow-hidden rounded-xl border border-white/20 bg-white/[0.008] p-6 transition-colors hover:border-white/30 hover:bg-white/[0.02]'>
        <div className='pointer-events-none absolute inset-0 bg-gradient-to-b from-white/[0.04] via-white/[0.01] to-transparent opacity-0 transition-opacity duration-300 group-hover:opacity-100' />

        <div className='relative z-10 mb-6'>
          <h3 className='mb-3 text-lg font-semibold text-white'>{title}</h3>
          <p className='text-sm leading-relaxed text-white/40'>{description}</p>
        </div>

        <div className='relative z-10 flex flex-1 items-center justify-center'>
          <div className='icon-spotlight-container flex h-16 w-16 items-center justify-center'>
            <Icon icon={icon} className='h-12 w-12 text-white/40' />
          </div>
        </div>
      </div>
    </a>
  );
}

export function FeaturesBentoBox() {
  const features = [
    {
      title: 'Long-Lived Stateful Compute',
      description:
        'Like AWS Lambda but with persistent memory and no timeouts. Your actors remember state between requests and intelligently hibernate when idle to save resources.',
      href: '/docs/actors',
      icon: faBolt,
      variant: 'large' as const
    },
    {
      title: 'Blazing-Fast Performance',
      description:
        'State stored on the same machine as compute. Ultra-fast reads/writes with no database round trips.',
      href: '/docs/actors/state',
      icon: faRocket,
      variant: 'medium' as const
    },
    {
      title: 'Built-in Realtime',
      description: 'WebSockets & SSE support out of the box. Update state and broadcast changes instantly.',
      href: '/docs/actors/events',
      icon: faBolt,
      variant: 'medium' as const
    },
    {
      title: 'Fault Tolerant',
      description: 'Built-in error handling & recovery',
      href: '/docs/actors/lifecycle',
      icon: faShieldHalved,
      variant: 'small' as const
    },
    {
      title: 'Auto-Hibernation',
      description: 'Actors sleep when idle, wake instantly on demand',
      href: '/docs/actors/lifecycle',
      icon: faMoon,
      variant: 'small' as const
    },
    {
      title: 'Scheduling',
      description: 'Persistent timeouts survive restarts and crashes',
      href: '/docs/actors/schedule',
      icon: faClock,
      variant: 'small' as const
    },
    {
      title: 'RPC & Events',
      description: 'Full-featured messaging system',
      href: '/docs/actors/actions',
      icon: faDatabase,
      variant: 'small' as const
    }
  ];

  return (
    <section className='w-full'>
      <div className='container relative mx-auto max-w-[1500px] px-6 lg:px-16 xl:px-20'>
        <h2 className='font-700 mb-8 text-center text-2xl text-white sm:text-3xl'>
          Each lightweight Actor comes packed with features
        </h2>
        <div className='grid auto-rows-[200px] grid-cols-1 gap-4 sm:grid-cols-2 md:grid-cols-4 md:gap-4 xl:gap-6'>
          {/* Large hero feature */}
          <FeatureCard
            title={features[0].title}
            description={features[0].description}
            href={features[0].href}
            icon={features[0].icon}
            className='sm:col-span-2 sm:row-span-2 md:col-span-2 md:row-span-2'
            variant='large'
          />

          {/* Medium features */}
          <FeatureCard
            title={features[1].title}
            description={features[1].description}
            href={features[1].href}
            icon={features[1].icon}
            className='sm:col-span-2 md:col-span-2'
            variant='medium'
          />

          <FeatureCard
            title={features[2].title}
            description={features[2].description}
            href={features[2].href}
            icon={features[2].icon}
            className='sm:col-span-2 md:col-span-2'
            variant='medium'
          />

          {/* Small features */}
          {features.slice(3).map(feature => (
            <FeatureCard
              key={feature.href}
              title={feature.title}
              description={feature.description}
              href={feature.href}
              icon={feature.icon}
              className='sm:col-span-1 md:col-span-1'
              variant='small'
            />
          ))}
        </div>
      </div>
    </section>
  );
}
