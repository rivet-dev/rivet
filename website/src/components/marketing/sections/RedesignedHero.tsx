'use client';

import { useState } from 'react';
import { Terminal, ArrowRight, Check } from 'lucide-react';
import { motion } from 'framer-motion';

const ThinkingImageCycler = ({ images }: { images: string[] }) => {
  const [currentIndex, setCurrentIndex] = useState(0);
  const [showFan, setShowFan] = useState(false);

  const handleClick = () => {
    setShowFan(false);
    setCurrentIndex((prev) => (prev + 1) % images.length);
  };

  const handleMouseEnter = () => {
    setShowFan(true);
    setTimeout(() => {
      setShowFan(false);
    }, 1000);
  };

  const handleMouseLeave = () => {
    setShowFan(false);
  };

  const getNextIndices = (count: number) => {
    const indices = [];
    for (let i = 1; i <= count; i++) {
      indices.push((currentIndex + i) % images.length);
    }
    return indices;
  };

  const fanCards = getNextIndices(3);

  return (
    <div
      className="relative w-[280px] h-[350px] sm:w-[400px] sm:h-[500px] cursor-pointer"
      onClick={handleClick}
      onMouseEnter={handleMouseEnter}
      onMouseLeave={handleMouseLeave}
    >
      {fanCards.map((imageIndex, i) => {
        const rotation = showFan ? (i + 1) * 6 : 0;
        const translateX = showFan ? (i + 1) * 15 : 0;
        const translateY = showFan ? (i + 1) * -5 : 0;
        const scale = 1 - (i + 1) * 0.02;

        return (
          <div
            key={`fan-${i}`}
            className="absolute inset-0 rounded-lg overflow-hidden shadow-xl transition-all duration-300 ease-out"
            style={{
              transform: `rotate(${rotation}deg) translateX(${translateX}px) translateY(${translateY}px) scale(${scale})`,
              zIndex: 3 - i,
              opacity: showFan ? 0.8 - i * 0.2 : 0,
            }}
          >
            <img
              src={images[imageIndex]}
              alt="Classical artwork depicting contemplation"
              loading="lazy"
              decoding="async"
              className="w-full h-full object-cover select-none pointer-events-none"
            />
          </div>
        );
      })}

      <div
        className="absolute inset-0 rounded-lg overflow-hidden shadow-2xl transition-transform duration-300 ease-out"
        style={{
          zIndex: 10,
          transform: showFan ? 'rotate(-3deg) translateX(-10px)' : 'rotate(0deg) translateX(0px)',
        }}
      >
        {images.map((src, index) => (
          <img
            key={src}
            src={src}
            alt="Classical artwork depicting contemplation and deep thought"
            loading={index === 0 ? 'eager' : 'lazy'}
            decoding="async"
            className={`absolute inset-0 w-full h-full object-cover transition-opacity duration-300 select-none pointer-events-none ${
              index === currentIndex ? 'opacity-100' : 'opacity-0'
            }`}
          />
        ))}
        <div className="absolute inset-0 bg-gradient-to-t from-black/60 via-transparent to-transparent" />
      </div>
    </div>
  );
};

const CopyInstallButton = () => {
  const [copied, setCopied] = useState(false);

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText('npx skills add rivet-dev/skills');
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch (err) {
      console.error('Failed to copy:', err);
    }
  };

  return (
    <div className='relative group w-full sm:w-auto'>
      <button
        onClick={handleCopy}
        className='w-full sm:w-auto inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md border border-white/20 px-4 py-2 text-sm text-zinc-300 transition-colors hover:border-white/30 hover:text-white'
      >
        {copied ? <Check className='h-4 w-4 text-green-500' /> : <Terminal className='h-4 w-4' />}
        npx skills add rivet-dev/skills
      </button>
      <div className='absolute left-1/2 -translate-x-1/2 top-full mt-4 opacity-0 translate-y-2 group-hover:opacity-100 group-hover:translate-y-0 transition-all duration-200 ease-out text-xs text-zinc-500 whitespace-nowrap pointer-events-none font-mono'>
        Give this to your coding agent
      </div>
    </div>
  );
};

interface RedesignedHeroProps {
  latestChangelogTitle: string;
  thinkingImages: string[];
}

export const RedesignedHero = ({ latestChangelogTitle, thinkingImages }: RedesignedHeroProps) => {
  return (
    <section className='relative flex h-[80vh] min-h-[600px] flex-col justify-center px-6'>
      <div className='mx-auto w-full max-w-7xl'>
        <div className='flex flex-col gap-12 lg:flex-row lg:items-center lg:justify-between lg:gap-32 xl:gap-48 2xl:gap-64'>
          <div className='max-w-xl'>
            <motion.div
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              transition={{ duration: 0.5, delay: 0.15 }}
              className='mb-6'
            >
              <a
                href='/changelog'
                className='inline-flex items-center gap-2 rounded-full bg-black/60 backdrop-blur-md border border-white/10 px-3 py-1.5 text-xs text-zinc-300 transition-colors hover:border-white/20 hover:text-white'
              >
                <span className='h-[4px] w-[4px] rounded-full bg-[#ff6030]' style={{ boxShadow: '0 0 2px #ffaa60, 0 0 4px #ff8040, 0 0 10px #ff6020, 0 0 20px rgba(255, 69, 0, 0.8)' }} />
                {latestChangelogTitle}
                <ArrowRight className='h-3 w-3' />
              </a>
            </motion.div>

            <motion.h1
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ duration: 0.5 }}
              className='mb-4 text-4xl font-normal leading-[1.1] tracking-tight text-white md:text-6xl'
            >
              The primitive for <br />
              software that thinks.
            </motion.h1>

            <motion.p
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ duration: 0.5, delay: 0.05 }}
              className='mb-6 text-lg text-zinc-400 md:text-xl'
            >
              Rivet Actors are a serverless primitive for stateful workloads.
            </motion.p>

            <motion.div
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ duration: 0.5, delay: 0.1 }}
              className='flex flex-col gap-3 sm:flex-row'
            >
              <a href='/docs'
                className='selection-dark inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md bg-white px-4 py-2 text-sm font-medium text-black transition-colors hover:bg-zinc-200'
              >
                Start Building
                <ArrowRight className='h-4 w-4' />
              </a>
              <CopyInstallButton />
            </motion.div>
          </div>

          <motion.div
            initial={{ opacity: 0, x: 20 }}
            animate={{ opacity: 1, x: 0 }}
            transition={{ duration: 0.8, delay: 0.2 }}
            className='flex-shrink-0 hidden lg:block'
          >
            <ThinkingImageCycler images={thinkingImages} />
          </motion.div>
        </div>

        {/* Mobile: Image */}
        <div className='lg:hidden mt-12'>
          <motion.div
            initial={{ opacity: 0, x: 20 }}
            animate={{ opacity: 1, x: 0 }}
            transition={{ duration: 0.8, delay: 0.2 }}
            className='flex justify-center'
          >
            <ThinkingImageCycler images={thinkingImages} />
          </motion.div>
        </div>
      </div>
    </section>
  );
};
