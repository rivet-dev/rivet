'use client';

import { useEffect, useState } from 'react';

// StyleInjector Component - Injects custom CSS
const StyleInjector = () => (
  <style
    dangerouslySetInnerHTML={{
      __html: `
    @import url('https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700;800&display=swap');
    
    /* --- Background Grid --- */
    .hero-background {
      position: absolute;
      top: 0;
      left: 0;
      right: 0;
      bottom: 0;
      width: 100%;
      height: 100%;
      background-color: #000000;
      background-image:
        linear-gradient(rgba(255, 255, 255, 0.05) 1px, transparent 1px),
        linear-gradient(90deg, rgba(255, 255, 255, 0.05) 1px, transparent 1px);
      background-size: 3rem 3rem;
      animation: pan-grid 60s linear infinite;
    }
    
    .hero-background-overlay {
      position: absolute;
      top: 0;
      left: 0;
      right: 0;
      bottom: 0;
      width: 100%;
      height: 100%;
      background: radial-gradient(ellipse 70% 40% at 50% 50%, rgba(255, 240, 220, 0.08) 0%, rgba(10, 10, 10, 1) 70%);
    }
    
    @keyframes pan-grid {
      0% { background-position: 0% 0%; }
      100% { background-position: 3rem 3rem; }
    }
    
    /* --- Request Packet Animations - L-shaped paths --- */
    /* Packets start at x=60px (icon center), need to reach x=280px (220px horizontal movement) */
    @keyframes fly-req-1 {
      0% { opacity: 0; transform: translate(0, 0); }
      2% { opacity: 1; }
      10% { transform: translate(0, 20px); } /* Drop down vertically - reach turn point very quickly */
      12% { transform: translate(0, 20px); } /* Brief pause at turn */
      100% { opacity: 0; transform: translate(220px, 20px); } /* Turn right and move all the way to actor */
    }
    
    @keyframes fly-req-2 {
      0% { opacity: 0; transform: translate(0, 0); }
      2% { opacity: 1; }
      100% { opacity: 0; transform: translate(220px, 0px); } /* Pure horizontal movement - no drop */
    }
    
    @keyframes fly-req-3 {
      0% { opacity: 0; transform: translate(0, 0); }
      2% { opacity: 1; }
      10% { transform: translate(0, -20px); } /* Move up - reach turn point very quickly */
      12% { transform: translate(0, -20px); } /* Brief pause at turn */
      100% { opacity: 0; transform: translate(220px, -20px); } /* Turn right and move all the way to actor */
    }
    
    /* --- Actor Processing Pulse --- */
    @keyframes pulse-actor {
      0% { box-shadow: 0 0 0 0 rgba(255, 69, 0, 0.3); }
      50% { box-shadow: 0 0 15px 8px rgba(255, 69, 0, 0.15); }
      100% { box-shadow: 0 0 0 0 rgba(255, 69, 0, 0); }
    }
    
    .animate-fly-1 { animation: fly-req-1 0.4s linear forwards; }
    .animate-fly-2 { animation: fly-req-2 0.4s linear forwards; }
    .animate-fly-3 { animation: fly-req-3 0.4s linear forwards; }
    .animate-pulse-actor { animation: pulse-actor 0.4s ease-in-out; }
    
    /* Green packet animation from actor to database */
    @keyframes fly-green {
      0% { opacity: 0; transform: translate(0, 0); }
      2% { opacity: 1; }
      100% { opacity: 0; transform: translate(0, 89px); } /* Move down 89px from actor bottom (221) to database (310) */
    }
    
    .animate-fly-green { animation: fly-green 0.6s linear forwards; }
    
    /* Database glow animation - triggers when ball reaches database */
    @keyframes glow-database {
      0% { box-shadow: 0 0 0 0 rgba(34, 197, 94, 0); }
      50% { box-shadow: 0 0 8px 4px rgba(34, 197, 94, 0.08); }
      100% { box-shadow: 0 0 0 0 rgba(34, 197, 94, 0); }
    }
    
    .animate-glow-database { 
      animation: glow-database 0.3s ease-out;
    }
    
    /* Connection lines */
    .connection-line {
      stroke: rgba(59, 130, 246, 0.5);
      stroke-width: 2.5;
      stroke-dasharray: 5, 5;
      animation: dash-move 2s linear infinite;
    }
    
    .connection-line-green {
      stroke: rgba(34, 197, 94, 0.6);
      stroke-width: 2.5;
      stroke-dasharray: 5, 5;
      animation: dash-move 2s linear infinite;
    }
    
    @keyframes dash-move {
      0% { stroke-dashoffset: 0; }
      100% { stroke-dashoffset: 10; }
    }
  `
    }}
  />
);

// UserNode Component
const UserNode = ({
  label,
  position,
  style
}: {
  label: string;
  position: string;
  style?: React.CSSProperties;
}) => (
  <div className={`absolute ${position} flex items-center space-x-3`} style={{ zIndex: 3, ...style }}>
    <div className='flex h-10 w-10 items-center justify-center rounded-full border-2 border-neutral-500 bg-neutral-700'>
      <svg
        className='h-5 w-5 text-neutral-400'
        fill='none'
        viewBox='0 0 24 24'
        strokeWidth={1.5}
        stroke='currentColor'
      >
        <path
          strokeLinecap='round'
          strokeLinejoin='round'
          d='M15.75 6a3.75 3.75 0 11-7.5 0 3.75 3.75 0 017.5 0zM4.501 20.118a7.5 7.5 0 0114.998 0A1.5 1.5 0 0118 21.75H6a1.5 1.5 0 01-1.499-1.632z'
        />
      </svg>
    </div>
    <span className='text-sm font-medium text-neutral-400'>{label}</span>
  </div>
);

// DatabaseNode Component - Green dot for database/API
const DatabaseNode = ({
  label,
  position,
  shouldGlow
}: {
  label: string;
  position: string;
  shouldGlow?: boolean;
}) => (
  <div className={`absolute ${position} flex items-center space-x-3`} style={{ zIndex: 3 }}>
    <div
      className={`flex h-10 w-10 items-center justify-center rounded-full border-2 border-green-400 bg-green-600 ${
        shouldGlow ? 'animate-glow-database' : ''
      }`}
    >
      <svg
        className='h-5 w-5 text-green-200'
        fill='none'
        viewBox='0 0 24 24'
        strokeWidth={1.5}
        stroke='currentColor'
      >
        <path
          strokeLinecap='round'
          strokeLinejoin='round'
          d='M20.25 6.375c0 2.278-3.694 4.125-8.25 4.125S3.75 8.653 3.75 6.375m16.5 0c0-2.278-3.694-4.125-8.25-4.125S3.75 4.097 3.75 6.375m16.5 0v11.25c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125V6.375m16.5 0v3.75m-16.5-3.75v3.75m16.5 0v3.75C20.25 16.153 16.556 18 12 18s-8.25-1.847-8.25-4.125v-3.75m16.5 0c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125'
        />
      </svg>
    </div>
    <span className='text-sm font-medium text-green-400'>{label}</span>
  </div>
);

// RequestPacket Component
const RequestPacket = ({
  id,
  status,
  position,
  startX,
  startY
}: {
  id: string;
  status: string;
  position: string;
  startX?: number;
  startY?: number;
}) => {
  let animationClass = '';
  if (status === 'flying') {
    if (id === 'req1') animationClass = 'animate-fly-1';
    if (id === 'req2') animationClass = 'animate-fly-2';
    if (id === 'req3') animationClass = 'animate-fly-3';
  }
  return (
    <div
      className={`absolute h-5 w-5 rounded-full border-2 border-blue-300 bg-blue-500 shadow-lg shadow-blue-500/50 ${
        position || ''
      } ${status === 'flying' ? animationClass : 'opacity-0'}`}
      style={{
        zIndex: 1, // Behind the grey icons (which are z-index: 3)
        left: startX ? `${startX - 10}px` : undefined,
        top: startY ? `${startY - 10}px` : undefined
      }}
    />
  );
};

// GreenPacket Component - travels from actor to database
const GreenPacket = ({ status }: { status: string }) => {
  return (
    <div
      className={`absolute h-5 w-5 rounded-full border-2 border-green-300 bg-green-500 shadow-lg shadow-green-500/50 ${
        status === 'processing' ? 'animate-fly-green' : 'opacity-0'
      }`}
      style={{
        zIndex: 1,
        left: '366px', // Actor center x=376, minus 10px for icon center
        top: '211px' // Actor bottom y=221, minus 10px for icon center
      }}
    />
  );
};

// ActorAnimation Component
const ActorAnimation = () => {
  const [actorState, setActorState] = useState({ count: 10 });
  const [actorStatus, setActorStatus] = useState<'hibernating' | 'active' | 'processing'>('hibernating');
  const [requests, setRequests] = useState({ req1: 'idle', req2: 'idle', req3: 'idle' });
  const [step, setStep] = useState(0);
  const [shouldGlowDatabase, setShouldGlowDatabase] = useState(false);

  useEffect(() => {
    const sequence = [
      () => {
        setActorStatus('active');
        setShouldGlowDatabase(false); // Reset glow when status changes
      },
      () => setRequests(r => ({ ...r, req1: 'flying' })),
      () => {
        setRequests(r => ({ ...r, req1: 'idle' }));
        setActorStatus('processing');
        setActorState(s => ({ count: s.count + 1 }));
        // Trigger database glow when ball reaches (after 0.6s)
        setShouldGlowDatabase(false); // Reset first
        setTimeout(() => setShouldGlowDatabase(true), 600);
      },
      () => {
        setActorStatus('active');
        setShouldGlowDatabase(false); // Reset glow when status changes
      },
      () => setRequests(r => ({ ...r, req2: 'flying' })),
      () => {
        setRequests(r => ({ ...r, req2: 'idle' }));
        setActorStatus('processing');
        setActorState(s => ({ count: s.count + 1 }));
        // Trigger database glow when ball reaches (after 0.6s)
        setShouldGlowDatabase(false); // Reset first
        setTimeout(() => setShouldGlowDatabase(true), 600);
      },
      () => {
        setActorStatus('active');
        setShouldGlowDatabase(false); // Reset glow when status changes
      },
      () => setRequests(r => ({ ...r, req3: 'flying' })),
      () => {
        setRequests(r => ({ ...r, req3: 'idle' }));
        setActorStatus('processing');
        setActorState(s => ({ count: s.count + 1 }));
        // Trigger database glow when ball reaches (after 0.6s)
        setShouldGlowDatabase(false); // Reset first
        setTimeout(() => setShouldGlowDatabase(true), 600);
      },
      () => {
        setActorStatus('hibernating');
        setShouldGlowDatabase(false); // Reset glow when status changes
      },
      () => {
        setActorState({ count: 10 });
        setStep(0);
      }
    ];

    const delays = [1000, 1200, 400, 500, 1200, 400, 500, 1200, 400, 3000, 500];

    const timer = setTimeout(() => {
      if (sequence[step]) {
        sequence[step]();
        setStep(s => s + 1);
      }
    }, delays[step] || 1000);

    return () => clearTimeout(timer);
  }, [step]);

  const getActorClasses = () => {
    let classes = 'w-48 h-48 border-2 flex flex-col items-center justify-center transition-all duration-500';

    if (actorStatus === 'hibernating') {
      classes += ' bg-orange-900/30 border-orange-500/20 opacity-60';
    } else if (actorStatus === 'active') {
      classes += ' bg-orange-900/80 border-orange-400 opacity-100';
    } else if (actorStatus === 'processing') {
      classes += ' bg-orange-900/80 border-orange-300 opacity-100 animate-pulse-actor';
    }

    return classes;
  };

  const getActorTextClasses = () => {
    let classes = 'transition-all duration-500';
    if (actorStatus === 'hibernating') {
      classes += ' text-orange-500/40';
    } else {
      classes += ' text-orange-300';
    }
    return classes;
  };

  return (
    <div className='relative flex h-[400px] w-full items-center justify-center lg:h-[500px]'>
      <div className='relative h-[350px] w-[600px]'>
        {/* SVG Connection Lines - Behind everything - L-shaped paths */}
        <svg className='pointer-events-none absolute inset-0 h-full w-full' style={{ zIndex: 0 }}>
          {/* L-shaped path from top node: down then right to actor top */}
          {/* Icon is 40px wide, center at 40px + 20px = 60px. Icon is 40px tall, center Y at top + 20px */}
          <path d='M 60 40 L 60 60 L 280 60' className='connection-line' fill='none' />
          {/* Horizontal path from middle node: straight to actor center (no drop) */}
          <path d='M 60 125 L 280 125' className='connection-line' fill='none' />
          {/* L-shaped path from bottom node: up then right to actor bottom */}
          <path d='M 60 210 L 60 190 L 280 190' className='connection-line' fill='none' />
          {/* Green dashed line from actor to database/API below */}
          {/* Actor: left edge at x=280, width 192px (w-48), so center at x=376. Top at y=125 with translateY(-50%), height 192px, so bottom at y=221 */}
          {/* Database icon center is at y=310 (350px height - 40px), and centered horizontally with actor at x=376 */}
          <path d='M 376 221 L 376 310' className='connection-line-green' fill='none' />
        </svg>

        {/* User Nodes on the left - positioned relative to blue lines */}
        {/* Top line starts at y=40, User Request should be above it */}
        {/* Middle line is at y=125, API Call centered on it */}
        {/* Bottom line ends at y=190, Background Job is below it (center at y=230, top at 210px) */}
        <UserNode label='User Request' position='left-10' style={{ top: '0px' }} />
        <UserNode label='API Call' position='left-10' style={{ top: '105px' }} />
        <UserNode label='Background Job' position='left-10' style={{ top: '210px' }} />

        {/* Actor on the right - lines connect at x=280, y=60, y=125, y=190 */}
        {/* Actor is 192px wide (w-48), so if lines connect at x=280, position actor so left edge is at x=280 */}
        <div
          className='absolute left-[280px] top-[125px]'
          style={{ transform: 'translateY(-50%)', zIndex: 2 }}
        >
          <div className={`relative ${getActorClasses()}`} style={{ borderRadius: '42px' }}>
            <div className='absolute top-3 rounded-full bg-orange-400/20 px-3 py-1 text-xs font-medium text-orange-300'>
              Actor
            </div>
            <div className='mt-4 w-[85%] rounded-lg bg-black/30 p-4'>
              <code className='text-sm'>
                <span className={getActorTextClasses()}>{'{'}</span>
                <span className='text-white/80'>"count"</span>
                <span className='text-white'>: </span>
                <span className='font-bold text-fuchsia-400'>{JSON.stringify(actorState.count)}</span>
                <span className={getActorTextClasses()}>{'}'}</span>
              </code>
            </div>
            <span className={`mt-2 text-xs font-medium ${getActorTextClasses()}`}>Status: {actorStatus}</span>
          </div>
        </div>

        {/* Database/API Node below the actor - centered with actor */}
        <div className='absolute bottom-0 left-[376px]' style={{ transform: 'translateX(-50%)', zIndex: 3 }}>
          <div className='flex items-center space-x-3'>
            <div
              className={`flex h-10 w-10 items-center justify-center rounded-full border-2 border-green-400 bg-green-600 ${
                shouldGlowDatabase ? 'animate-glow-database' : ''
              }`}
            >
              <svg
                className='h-5 w-5 text-green-200'
                fill='none'
                viewBox='0 0 24 24'
                strokeWidth={1.5}
                stroke='currentColor'
              >
                <path
                  strokeLinecap='round'
                  strokeLinejoin='round'
                  d='M20.25 6.375c0 2.278-3.694 4.125-8.25 4.125S3.75 8.653 3.75 6.375m16.5 0c0-2.278-3.694-4.125-8.25-4.125S3.75 4.097 3.75 6.375m16.5 0v11.25c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125V6.375m16.5 0v3.75m-16.5-3.75v3.75m16.5 0v3.75C20.25 16.153 16.556 18 12 18s-8.25-1.847-8.25-4.125v-3.75m16.5 0c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125'
                />
              </svg>
            </div>
            <span className='text-sm font-medium text-green-400'>Database/API</span>
          </div>
        </div>

        {/* Request Packets - Start at center of user node icons, follow lines */}
        {/* Positioned at icon centers: top at y=40, middle at y=125, bottom at y=210 */}
        <RequestPacket id='req1' status={requests.req1} position='' startX={60} startY={40} />
        <RequestPacket id='req2' status={requests.req2} position='' startX={60} startY={125} />
        <RequestPacket id='req3' status={requests.req3} position='' startX={60} startY={210} />

        {/* Green Packet - travels from actor to database when processing */}
        <GreenPacket status={actorStatus} />
      </div>
    </div>
  );
};

export function NewHeroSection() {
  return (
    <>
      <StyleInjector />
      <section className='relative flex min-h-screen w-full items-center justify-center overflow-hidden text-white'>
        {/* Background Grid */}
        <div className='hero-background' aria-hidden='true'></div>
        <div className='hero-background-overlay' aria-hidden='true'></div>

        {/* Content */}
        <div className='container relative z-10 mx-auto px-6 py-20'>
          <div className='grid grid-cols-1 items-center gap-16 lg:grid-cols-2'>
            {/* Left: Text Content - Left Aligned */}
            <div className='flex flex-col space-y-8 text-left'>
              <h1
                className='font-heading text-5xl font-bold tracking-tighter md:text-6xl lg:text-7xl'
                style={{ color: '#FAFAFA' }}
              >
                <span className='block overflow-hidden leading-[1.1]'>
                  <span className='block animate-hero-line opacity-0' style={{ animationDelay: '0.1s' }}>
                    Stateful Backends.
                  </span>
                </span>
                <span className='block overflow-hidden leading-[1.1]'>
                  <span className='block animate-hero-line opacity-0' style={{ animationDelay: '0.3s' }}>
                    Finally Solved.
                  </span>
                </span>
              </h1>

              <p
                className='max-w-lg animate-hero-p text-lg opacity-0 md:text-xl'
                style={{ color: '#A0A0A0' }}
              >
                Rivet is open-source infrastructure for long-lived, in-memory processes called Actors. It's
                what you reach for when you hit the limits of Lambda.
              </p>

              <div className='flex animate-hero-cta flex-col gap-4 opacity-0 sm:flex-row'>
                <a href='/docs/quickstart/'
                  className='rounded-lg px-8 py-3.5 text-center font-bold text-white transition-all duration-200 hover:-translate-y-0.5 hover:bg-orange-600 hover:shadow-lg'
                  style={{ backgroundColor: '#FF4500' }}
                >
                  Quickstart – 5 Mins
                </a>
                <a href='https://github.com/rivet-gg/rivet'
                  className='rounded-lg px-8 py-3.5 text-center font-medium transition-all duration-200'
                  style={{
                    border: '1px solid #252525',
                    color: '#A0A0A0',
                    backgroundColor: 'rgba(37, 37, 37, 0.5)'
                  }}
                >
                  View on GitHub →
                </a>
              </div>
            </div>

            {/* Right: Animation */}
            <div className='flex h-full w-full items-center justify-center lg:justify-end'>
              <ActorAnimation />
            </div>
          </div>
        </div>
      </section>
    </>
  );
}
