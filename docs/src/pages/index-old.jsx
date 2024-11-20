'use client';
import React from 'react';
import { useState, useRef, useEffect } from 'react';
import CodeSection from '@/components/CodeSection';
import ProductSection from '@/components/ProductSection';
import MainFeatures from '@/components/MainFeatures';
import Earth from '@/components/Earth';
import { Button } from '@/components/Button';
import clsx from 'clsx';
import { motion, useAnimation } from 'framer-motion';
import IncludedSection from '@/components/Included';
import LevelUpSection from '@/components/LevelUpSection';
import grainDark from '@/images/effects/grain-dark.png';
import RotatingText from '@/components/RotatingText';
import {
  faArrowDown,
  faArrowRight,
  faBook,
  faLock,
  faServer,
  faFileCertificate,
  faCodeBranch,
  faAlien8bit,
  faSkullCrossbones,
  faSkull,
  faCode,
  faShield,
  faAddressCard,
  faChessKnight,
  faBug,
  faPlus,
  faGears,
  iconPack,
  faGithub,
  Icon
} from '@rivet-gg/icons';
import { Ferris } from '../components/icons/Ferris';

// https://github.com/FortAwesome/Font-Awesome/issues/19348
const { library } = require('@fortawesome/fontawesome-svg-core');
library.add(iconPack);

function camelToKebab(str) {
  return str.replace(/([a-z])([A-Z])/g, '$1-$2').toLowerCase();
}

function kebabToUpperCamel(str) {
  return str
    .split('-')
    .map(word => word.charAt(0).toUpperCase() + word.slice(1))
    .join('');
}

export default function Index() {
  const restOfPageControls = useAnimation();

  useEffect(() => {
    const timer = setTimeout(() => {
      restOfPageControls.start({ opacity: 1 });
    }, 300); // Start fading in the rest of the page 0.65 seconds after component mount

    return () => clearTimeout(timer);
  }, [restOfPageControls]);

  return (
    <div>
      <div className='relative isolate overflow-x-hidden'>
        <div className='pointer-events-none relative'>
          <div className='h-16 sm:h-28' />
          <Title />
          <div className='h-32 sm:h-60' />
        </div>

        <motion.div initial={{ opacity: 0 }} animate={restOfPageControls} transition={{ duration: 0.325 }}>
          <div className='relative border-t-2 border-cream-100/10 py-16'>
            {/* Background */}
            <div
              style={{ backgroundImage: `url(${grainDark.src})`, opacity: 0.4 }}
              className='pointer-events-none absolute inset-0 -z-20 bg-repeat transition'></div>
            <div className='pointer-events-none absolute inset-0 -z-10 bg-gradient-to-b from-transparent to-[#090909] opacity-100'></div>

            <MainFeatures />

            <div className='h-44' />
          </div>

          <PoweringPlay />

          <div className='h-48 md:h-64' />

          <ProductSection />

          <div className='h-56 md:h-96' />

          <CodeSection />

          <div className='h-48 md:h-96' />

          <IncludedSection />

          <div className='h-96' />

          <div className='main-content-container mx-auto px-6'>
            <AdaptableSection />
          </div>

          <div className='h-60' />

          <Philosophy />

          <div className='h-32'></div>

          <LevelUpSection />
        </motion.div>
      </div>
    </div>
  );
}

function Title() {
  // const subtitleContent = [
  //   { subtle: "Build painlessly with " },
  //   { feature: "Game Servers", href: "https://www.google.com", icon: faServer },
  //   { subtle: " and " },
  //   { feature: "Matchmaking", href: "https://www.google.com", icon: faChessKnight },
  //   { subtle: "." },
  //   { break: true },
  //   { subtle: "Launch quickly using " },
  //   { feature: "Authentication", href: "https://www.google.com", icon: faAddressCard },
  //   { subtle: " and " },
  //   { feature: "Backend Scripting", href: "https://www.google.com", icon: faCode },
  //   { subtle: "." },
  //   { break: true },
  //   { subtle: "Scale effortlessly with " },
  //   { feature: "DDoS Mitigation", href: "https://www.google.com", icon: faShield },
  //   { subtle: " and " },
  //   { feature: "Monitoring", href: "https://www.google.com", icon: faChartLine },
  //   { subtle: "." }
  // ];

  // const subtitleContent = [
  //   { subtle: "Build multiplayer painlessly with " },
  //   { feature: "Game Servers", href: "https://www.google.com", icon: faServer },
  //   { subtle: ", " },
  //   { feature: "Matchmaking", href: "https://www.google.com", icon: faChessKnight },
  //   { subtle: ", and " },
  //   { feature: "Authentication", href: "https://www.google.com", icon: faAddressCard },
  //   { subtle: "." },
  //   { break: true },
  //   { subtle: "Customize endlessly using " },
  //   { feature: "Backend Scripting", href: "https://www.google.com", icon: faCode },
  //   { subtle: "." },
  //   { break: true },
  //   { subtle: "Scale effortlessly with " },
  //   { feature: "DDoS Mitigation", href: "https://www.google.com", icon: faShield },
  //   { subtle: " and " },
  //   { feature: "Monitoring", href: "https://www.google.com", icon: faChartLine },
  //   { subtle: "." }
  // ];

  const subtitleContent = [
    { subtle: 'Build multiplayer painlessly with ' },
    { feature: 'Game Servers', href: '/modules', icon: faServer },
    { subtle: ', ' },
    { feature: 'Matchmaking', href: '/modules', icon: faChessKnight },
    { subtle: ',' },
    { subtle: ' and ' },
    { break: true },
    {
      feature: 'Authentication',
      href: '/modules',
      target: '_blank',
      icon: faAddressCard
    },
    { subtle: '. ' },
    { subtle: 'Customize endlessly using ' },
    {
      feature: 'Backend Scripting',
      href: '/modules',
      target: '_blank',
      icon: faCode
    },
    { subtle: '.' },
    { break: true },
    { subtle: 'Scale effortlessly with ' },
    {
      feature: 'DDoS Mitigation',
      href: '/modules',
      icon: faShield
    },
    { subtle: ' and ' },
    { feature: 'Monitoring', href: '/modules', icon: faBug },
    { subtle: '.' }
  ];

  return (
    <div className='flex w-full flex-col items-center justify-center px-4 text-center'>
      <div className='relative flex flex-col items-center justify-center'>
        {/* BG gradient to cover the game */}
        <div
          className='pointer-events-none absolute inset-x-[-100px] inset-y-[-50px] -z-10'
          style={{
            background: 'radial-gradient(ellipse at center, rgba(9, 9, 9, 0.8) 0%, rgba(9, 9, 9, 0) 70%)',
            backgroundSize: '100% 100%'
          }}
        />

        <motion.h1
          initial={{ opacity: 0, y: 10, scale: 0.95 }}
          animate={{ opacity: 1, y: 0, scale: 1 }}
          transition={{ duration: 0.325, delay: 0.135 }}
          className={clsx(
            'mb-2 mt-8 text-center font-display font-extrabold tracking-tight text-cream-100',
            'gap-3',
            'text-3xl',
            'sm:text-5xl',
            'lg:text-6xl'
          )}>
          Open-Source Multiplayer Tooling
        </motion.h1>

        <div className='h-6' />

        <motion.div
          initial={{ opacity: 0, y: 10, scale: 0.95 }}
          animate={{ opacity: 1, y: 0, scale: 1 }}
          transition={{ duration: 0.325, delay: 0.27 }}
          className={clsx(
            'text-center text-justify font-display text-cream-100 sm:text-center',
            'text-xl',
            'md:text-2xl',
            'lg:text-3xl'
          )}>
          {subtitleContent.map((item, index) => {
            if ('feature' in item) {
              return (
                <a
                  key={index}
                  href={item.href}
                  target={item.target}
                  className={clsx(
                    'pointer-events-auto font-bold text-[#D6CFC4]',
                    'underline decoration-transparent',
                    'hover:text-orange-500 hover:decoration-orange-500',
                    'transition-all duration-100'
                  )}>
                  <Icon
                    icon={item.icon}
                    className={clsx(
                      'text-base md:text-lg lg:text-xl',
                      'ml-0.5 mr-0.5 sm:ml-1 sm:mr-1',
                      'lg:mb-0.75 mb-0.5'
                    )}
                  />{' '}
                  {item.feature}
                </a>
              );
            } else if ('subtle' in item) {
              return (
                <span key={index} className='text-[#837E77]'>
                  {item.subtle}
                </span>
              );
            } else if ('break' in item) {
              return (
                <>
                  <br key={index} className='hidden sm:block' />
                  <span className='sm:hidden'> </span>
                </>
              );
            }
          })}
        </motion.div>

        <div className='h-5' />

        <motion.div
          initial={{ opacity: 0, y: 10, scale: 0.95 }}
          animate={{ opacity: 1, y: 0, scale: 1 }}
          transition={{ duration: 0.325, delay: 0.405 }}
          className={clsx(
            'italic text-orange-500',
            'text-center font-display tracking-tight',
            'text-center text-xl',
            'sm:text-2xl',
            'lg:text-3xl'
          )}>
          Open-Source & Self-Hostable.
        </motion.div>

        <div className='h-6' />

        <motion.div
          initial={{ opacity: 0, y: 10, scale: 0.95 }}
          animate={{ opacity: 1, y: 0, scale: 1 }}
          transition={{ duration: 0.325, delay: 0.54 }}
          className={clsx(
            'flex items-center justify-center',
            'gap-1 px-3 py-1',
            'text-center text-xs font-bold sm:text-sm',
            'text-cream-100',
            'rounded-full border border-cream-100/20',
            'opacity-70 hover:opacity-100',
            'hover:border-orange-500 hover:bg-orange-500/10',
            'pointer-events-auto'
          )}>
          <GitHubStars />
        </motion.div>
      </div>

      <div className='h-24' />

      <motion.div
        initial={{ opacity: 0, y: 10, scale: 0.95 }}
        animate={{ opacity: 1, y: 0, scale: 1 }}
        transition={{ duration: 0.325, delay: 0.675 }}
        className='relative flex flex-col items-center justify-center'>
        {/* BG gradient to cover the game */}
        <div
          className='pointer-events-none absolute inset-x-[-100px] inset-y-[-50px] -z-10'
          style={{
            background: 'radial-gradient(ellipse at center, rgba(9, 9, 9, 0.8) 0%, rgba(9, 9, 9, 0) 70%)',
            backgroundSize: '100% 100%'
          }}
        />

        <motion.div
          initial={{ opacity: 0, y: 10 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.325, delay: 0.48 }}
          className='text-md font-semibold text-cream-100/90'>
          Install Plugin
        </motion.div>

        <div className='h-1' />

        <motion.div
          initial={{ opacity: 0, y: 10 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.325, delay: 0.555 }}
          className='text-sm text-cream-100/70'>
          Create & deploy a multiplayer game from scratch in under 5 minutes.
        </motion.div>

        <div className='h-4' />

        <div className='grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-4'>
          <motion.div
            initial={{ opacity: 0, y: 10, scale: 0.95 }}
            animate={{ opacity: 1, y: 0, scale: 1 }}
            transition={{ duration: 0.325, delay: 0.63 }}>
            <DownloadButton title='Unity' href='https://github.com/rivet-gg/plugin-unity' />
          </motion.div>
          <motion.div
            initial={{ opacity: 0, y: 10, scale: 0.95 }}
            animate={{ opacity: 1, y: 0, scale: 1 }}
            transition={{ duration: 0.325, delay: 0.705 }}>
            <DownloadButton title='Godot' href='https://godotengine.org/asset-library/asset/1881' />
          </motion.div>
          <motion.div
            initial={{ opacity: 0, y: 10, scale: 0.95 }}
            animate={{ opacity: 1, y: 0, scale: 1 }}
            transition={{ duration: 0.325, delay: 0.78 }}>
            <DownloadButton title='Unreal Engine' href='https://github.com/rivet-gg/plugin-unreal' />
          </motion.div>
          <motion.div
            initial={{ opacity: 0, y: 10, scale: 0.95 }}
            animate={{ opacity: 1, y: 0, scale: 1 }}
            transition={{ duration: 0.325, delay: 0.855 }}>
            <DownloadButton title='HTML5 & Other' href='/docs/html5/tutorials/quickstart' icon={faBook} />
          </motion.div>
        </div>
      </motion.div>
    </div>
  );
}

function DownloadButton({ title, href, icon = faArrowDown }) {
  const isExternalLink = href.startsWith('http') || href.startsWith('//');
  const linkProps = isExternalLink ? { target: '_blank', rel: 'noopener noreferrer' } : {};
  const [mousePosition, setMousePosition] = useState({ x: -1000, y: -1000 });
  const [isHovered, setIsHovered] = useState(false);
  const buttonRef = useRef(null);

  useEffect(() => {
    const handleMouseMove = event => {
      if (buttonRef.current) {
        const rect = buttonRef.current.getBoundingClientRect();
        setMousePosition({
          x: event.clientX - rect.left,
          y: event.clientY - rect.top
        });
      }
    };

    document.addEventListener('mousemove', handleMouseMove);

    return () => {
      document.removeEventListener('mousemove', handleMouseMove);
    };
  }, []);

  return (
    <a
      ref={buttonRef}
      href={href}
      {...linkProps}
      variant='primaryJuicy'
      className={clsx(
        'h-11 w-48',
        'pl-2',
        'relative overflow-hidden',
        'pointer-events-auto',
        // Base styles
        'inline-flex items-stretch justify-center',
        'rounded',
        'text-sm font-bold',
        'bg-charcole-900/30 text-cream-100',
        'border-2 border-cream-100/5',
        // Animation
        'transition-[background,transform,border-color,color,box-shadow] duration-200 ease-out',
        'hover:-translate-y-[2px] hover:shadow-[0_4px_10px_0_rgba(0,0,0,0.6)]',
        'active:opacity-75',
        // Hover
        'hover:border-cream-100/20 hover:bg-charcole-800/50 hover:text-cream-50',
        // Selected
        'aria-selected:border-cream-100/30 aria-selected:text-cream-50',
        // Disabled
        'disabled:border-cream-100 disabled:opacity-60 disabled:hover:bg-transparent disabled:hover:text-cream-100',
        // Loading
        'aria-busy:translate-y-0 aria-busy:border-neutral-300 aria-busy:hover:bg-transparent aria-busy:hover:text-white'
      )}
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}>
      <div className='flex flex-grow items-center justify-center'>{title}</div>
      <div className='my-auto h-7 w-[2px] bg-cream-100/5' />
      <div className={clsx('flex-0 w-11', 'flex items-center justify-center')}>
        <Icon icon={icon} />
      </div>

      {/* Gloss effect */}
      <div
        className={clsx(
          'pointer-events-none absolute left-0 top-0 h-full w-full',
          'transition-opacity duration-300',
          isHovered ? 'opacity-100' : 'opacity-90'
        )}
        style={{
          background: `radial-gradient(circle 300px at ${mousePosition.x}px ${mousePosition.y}px, rgba(229,231,235,0.1), transparent)`
        }}
      />
    </a>
  );
}

function GlowVideo({ style, ...props }) {
  const videoRef = useRef(null);
  return (
    <video autoPlay loop muted playsInline ref={videoRef} {...props}>
      <source src='https://assets2.rivet.gg/effects/glow.webm' type='video/webm' />
    </video>
  );
}

function GitHubStars({ repo = 'rivet-gg/rivet', ...props }) {
  const [stars, setStars] = useState(0);
  const [isHovered, setIsHovered] = useState(false);

  useEffect(() => {
    const fetchData = async () => {
      try {
        const response = await fetch(`https://api.github.com/repos/${repo}`);
        const data = await response.json();
        setStars(data.stargazers_count);
      } catch (err) {
        console.error('Failed to fetch stars', err);
      }
    };

    fetchData();
  }, [repo]);

  return (
    <a
      href={`https://github.com/${repo}`}
      target='_blank'
      rel='noreferrer'
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
      {...props}>
      {isHovered ? (
        <>
          <Icon icon={faSkullCrossbones} /> Pirate our source code{' '}
          <Icon icon={faArrowRight} className='h-6 w-6' />
        </>
      ) : (
        <>
          <Icon icon={faGithub} /> {stars ? <>{formatNumber(stars)} stars</> : <>GitHub</>}{' '}
          <Icon icon={faArrowRight} className='h-6 w-6' />
        </>
      )}
    </a>
  );
}

function formatNumber(num) {
  if (num >= 1000) {
    return (num / 1000).toFixed(1) + 'k';
  } else {
    return num.toString();
  }
}

const PHILOSOPHY_ITEMS = [
  { icon: faFileCertificate, title: 'Permissive License (Apache 2.0)' },
  { icon: faLock, title: 'Audit security' },
  { icon: faServer, title: 'Optionally self-host on-premise' },
  { iconEl: <Ferris className='h-6 w-6' />, title: '100% crustacean-certified Rust' },
  { icon: faSkull, title: 'Trust no-one, own your backend', classes: 'font-psychotic' }
];

function Philosophy() {
  return (
    <div className='main-content-container flex flex-col items-center py-20 md:py-40'>
      <div
        className={clsx(
          'relative',
          'border-4 border-cream-100/5',
          'mx-4',
          'sm:px-16 sm:pb-14 sm:pt-16',
          'px-6 pb-6 pt-6',
          'rounded-md'
        )}>
        {/* BG */}
        <div
          style={{ backgroundImage: `url(${grainDark.src})`, opacity: 0.2 }}
          className='pointer-events-none absolute inset-0 -z-20 bg-repeat transition'></div>

        {/* Title */}
        <div className='mx-auto max-w-4xl'>
          <h2 className='font-display text-5xl font-bold tracking-tight text-cream-100'>
            Our commitment to open-source <Icon icon={faCodeBranch} className='ml-3 text-4xl' />
          </h2>
        </div>

        {/* Details */}
        <div className='mt-8 flex max-w-2xl flex-col gap-4 text-center text-justify text-cream-100/80'>
          <p>
            Everyone who works at Rivet has shipped a multiplayer game. We{"'"}ve all experienced how much
            time & money is required to ship a game, and how much harder it is to maintain it.
          </p>
          <p>
            We refused to use closed-source solutions that locked us in and failed to grow alongside our use
            cases, so we always opted to build solutions ourselves. To build the tool we needed, we knew it
            had to make it radically open-source.
          </p>
          <p>The future of game development is open-source and we{"'"}re here to lead the way.</p>
        </div>

        <div className='mt-8 flex max-w-2xl flex-col items-stretch gap-4'>
          {PHILOSOPHY_ITEMS.map((item, i) => (
            <div key={i} className='flex flex-row items-center gap-3 font-semibold text-cream-100'>
              <div className='flex h-9 w-9 items-center justify-center rounded-lg bg-white/[4%] outline outline-1 outline-white/[8%]'>
                {item.icon && <Icon icon={item.icon} className='w-4' />}
                {item.iconEl && item.iconEl}
              </div>
              <span className={item.classes ?? ''}>{item.title}</span>
            </div>
          ))}
        </div>

        <div className='mt-8 flex justify-center sm:mt-12'>
          <GitHubStars className='font-semibold text-white/50 hover:text-white' />
        </div>
      </div>
    </div>
  );
}

function EngineGrid() {
  return (
    <div className='bg-black p-8 text-white'>
      <div className='mx-auto max-w-4xl'>
        <div className='grid auto-rows-fr grid-cols-3 text-center'>
          {/* Grid Item 1 */}
          <div className='flex flex-col justify-center border border-white p-4'>
            <h3 className='mt-2 font-display text-5xl'>Unity</h3>
          </div>

          {/* Grid Item 2 */}
          <div className='flex flex-col justify-center border border-white p-4'>
            <h3 className='font-display text-purple-800 sm:text-7xl'>
              <Icon icon={faAlien8bit} />
            </h3>
          </div>

          {/* Grid Item 3 */}
          <div className=''>
            <h2 className='text-left font-display text-5xl font-bold'>Get started with Your Engine.</h2>
          </div>

          {/* Grid Item 4 */}
          <div className='flex flex-col justify-center border border-white p-4'>
            <h3 className='font-display text-purple-800 sm:text-7xl'>
              <Icon icon={faAlien8bit} />
            </h3>
          </div>

          {/* Grid Item 5 */}
          <div className='flex flex-col justify-center border border-white p-4'>
            <h3 className='mt-2 font-display text-5xl'>Godot</h3>
          </div>

          {/* Grid Item 6 */}
          <div className='flex flex-col justify-center border border-white p-4'>
            <h3 className='font-display text-purple-800 sm:text-7xl'>
              <Icon icon={faAlien8bit} />
            </h3>
          </div>

          {/* Grid Item 7 */}
          <div className='flex flex flex-col flex-col justify-center justify-center border border-white p-4'>
            <h3 className='mt-2 font-display text-5xl'>Unreal</h3>
          </div>

          {/* Grid Item 8 */}
          <div className='flex flex-col justify-center border border-white p-4'>
            <h3 className='mt-2 font-display text-5xl'>HTML5</h3>
          </div>

          {/* Grid Item 9 */}
          <div className='flex flex-col justify-center border border-white p-4'>
            <h3 className='mt-2 font-display text-5xl'>Custom</h3>
          </div>
        </div>
      </div>
    </div>
  );
}

const variants = {
  enter: direction => {
    return {
      x: direction > 0 ? 1000 : -1000,
      opacity: 0
    };
  },
  center: {
    zIndex: 1,
    x: 0,
    opacity: 1
  },
  exit: direction => {
    return {
      zIndex: 0,
      x: direction < 0 ? 1000 : -1000,
      opacity: 0
    };
  }
};

const swipeConfidenceThreshold = 10000;
const swipePower = (offset, velocity) => {
  return Math.abs(offset) * velocity;
};

function paginate(page, dir, arr) {
  const newPage = page + dir;
  if (newPage < 0) return { index: arr.length - (-newPage % arr.length), dir };
  return { index: newPage % arr.length, dir };
}

const PlaySessionsCounter = () => {
  const [currentTime, setCurrentTime] = useState(Date.now());
  const [hasMounted, setHasMounted] = useState(false);

  const updateClock = () => {
    let time = (21126202 / 30 / 24 / 60 / 60 / 1000) * (Date.now() - 1640995200000);
    setCurrentTime(Math.round(time));
  };

  useEffect(() => {
    setHasMounted(true);

    updateClock();

    const interval = setInterval(() => {
      updateClock();
    }, 50);

    return () => clearInterval(interval);
  }, []);

  if (!hasMounted) {
    return null;
  }

  const formattedTime = currentTime.toLocaleString();
  const timeElements = formattedTime.split('').map((char, index) => {
    const spanClass = isDigit(char) ? 'inline-block w-[0.52em]' : 'inline-block';
    return (
      <span key={index} className={`${spanClass} inline-block text-right`}>
        {char}
      </span>
    );
  });

  return <span>{timeElements}</span>;
};

function isDigit(char) {
  const digits = ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'];
  return digits.indexOf(char) !== -1;
}

const GAME_GENRES = [
  'FPS',
  'MOBA',
  'battle royale',
  'MMO',
  'racing',
  'RTS',
  'turn-based',
  'party',
  'mobile',
  'console',
  'web',
  'casual'
];

function PoweringPlay() {
  return (
    <div
      className={clsx(
        'relative flex h-[60vh] items-center justify-center md:h-[70vh]',
        'border-b-2 border-cream-100/10'
      )}>
      {/* Background earth */}
      <div className='absolute inset-0 -z-10 overflow-hidden bg-black'>
        <Earth className='absolute left-0 top-0 h-full w-full object-cover object-top' />
        {/* <div className='absolute inset-0 bg-gradient-to-b from-transparent from-80% to-black'></div> */}
        <div className='absolute inset-0 bg-gradient-to-t from-transparent from-80% to-charcole-950'></div>
      </div>

      {/* Content */}
      <h3
        className={clsx(
          'text-center font-display tracking-tight text-cream-100',
          'text-2xl sm:text-3xl md:text-4xl lg:text-5xl xl:text-6xl',
          'drop-shadow-[0_0_25px_rgba(0,0,0,0.9)]'
        )}>
        {/* Online indicator */}
        <div
          className={clsx(
            'relative inline-block flex-none rounded-full bg-orange-500',
            "before:absolute before:inset-0 before:animate-ping before:rounded-full before:bg-orange-500 before:opacity-70 before:content-['']",
            'mb-0.5 mr-2.5 h-3 w-3',
            'xs:mb-0.5 xs:mr-3 xs:h-4 xs:w-4',
            'sm:mb-0.5 sm:mr-4 sm:h-5 sm:w-5',
            'md:mb-0.75 md:mr-5 md:h-6 md:w-6',
            'lg:mb-1.5 lg:mr-6 lg:h-7 lg:w-7',
            'xl:mb-2 xl:mr-7 xl:h-8 xl:w-8'
          )}></div>
        <span className='opacity-75'>Powering</span> <PlaySessionsCounter />{' '}
        <span className='opacity-75'>play sessions</span>
        <br className='sm:hidden' /> <span className='opacity-75'>for</span>{' '}
        <RotatingText texts={GAME_GENRES} /> <span className='opacity-75'>games</span>
      </h3>
    </div>
  );
}

function AdaptableSection() {
  return (
    <div className='mx-auto flex max-w-6xl flex-col px-6 lg:px-8'>
      <h2 className='text-center font-display text-4xl font-bold tracking-tight text-cream-100 sm:text-5xl'>
        {'Need even more customization?'}
      </h2>
      <div className={clsx('mt-16 grid w-full gap-4', 'grid-cols-1 md:grid-cols-2')}>
        <AdaptableFeature
          title='Custom modules without server hassels'
          description='Write backend modules with TypeScript, Postgres, and real-time actors. Auto-generate SDKs for your module to use in engine. Reuse modules across multiple games with registries.'
          docsHref='/docs/modules/build/overview'
        />
        <AdaptableFeature
          title='Access to low-level APIs'
          description='Build with low-level APIs for highly custom use cases. Includes APIs for provisioning servers, customizing networking, managing builds, and tuning DDoS protection rules.'
        />
        <AdaptableFeature
          title='Fully automatable cloud'
          description="Build custom deploy pipelines using Rivet's cloud APIs. Anything you can do via a GUI is available as an API & CLI."
          docsHref='/docs/cloud'
        />
        <AdaptableFeature
          title='Integrated with existing tools'
          description='Works with your favorite tools & existing backends. Integrate Rivet incrementally without having to rewrite anything.'
          docsHref='/modules'
        />
      </div>
    </div>
  );
}

function AdaptableFeature({ title, description, docsHref, ...props }) {
  return (
    <div
      className={clsx(
        'relative border-4 border-cream-100/5 px-6 py-4 text-cream-100',
        'flex flex-col gap-4',
        'rounded-md'
      )}
      {...props}>
      {/* BG */}
      <div
        style={{ backgroundImage: `url(${grainDark.src})`, opacity: 0.2 }}
        className='pointer-events-none absolute inset-0 -z-20 bg-repeat transition'></div>

      {/* Content */}
      <div className='font-display text-3xl font-bold tracking-tight text-cream-100'>{title}</div>
      <p>{description}</p>
      <div className='flex-grow' />

      {/* Documentation */}
      {docsHref && (
        <a
          href={docsHref}
          target={docsHref.startsWith('http') ? '_blank' : undefined}
          rel={docsHref.startsWith('http') ? 'noreferrer' : undefined}
          className='flex items-center gap-1 text-xs font-bold text-orange-400 hover:text-orange-300 sm:text-sm'>
          Documentation
          <Icon icon={faArrowRight} className='h-6 w-6' />
        </a>
      )}
    </div>
  );
}

Index.description = 'Open-Source Multiplayer Tooling. A Single Tool to Manage Your Game Servers & Backend.';
Index.prose = false;
Index.fullWidth = true;

