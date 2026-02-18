'use client';

import { useEffect, useState } from 'react';
import { Terminal, ArrowRight, Check } from 'lucide-react';
import { AnimatePresence, motion } from 'framer-motion';

interface ThinkingImage {
  src: string;
  title: string;
  artist: string;
  date: string;
}

const ThinkingImageCycler = ({ images }: { images: ThinkingImage[] }) => {
  const [currentIndex, setCurrentIndex] = useState(0);
  const [showFan, setShowFan] = useState(false);
  const [leavingCards, setLeavingCards] = useState<Array<{ id: string; image: ThinkingImage }>>([]);

  useEffect(() => {
    // Preload upcoming images to avoid decode flashes during cycling.
    const preloadAhead = Math.min(4, images.length - 1);
    for (let i = 1; i <= preloadAhead; i++) {
      const next = images[(currentIndex + i) % images.length];
      const img = new window.Image();
      img.src = next.src;
    }
  }, [currentIndex, images]);

  const handleClick = () => {
    const leavingImage = images[currentIndex];
    setLeavingCards((prev) => [...prev, { id: `${leavingImage.src}-${Date.now()}`, image: leavingImage }]);
    setCurrentIndex((prev) => (prev + 1) % images.length);
  };

  const handleMouseEnter = () => {
    setShowFan(true);
  };

  const handleMouseLeave = () => {
    setShowFan(false);
  };

  const getStackIndices = (count: number) => {
    const indices = [];
    for (let i = 0; i < count; i++) {
      indices.push((currentIndex + i) % images.length);
    }
    return indices;
  };

  const getStackPose = (position: number, expanded: boolean) => {
    const basePoses = [
      { x: 0, y: 0, rotate: -0.7, scale: 1 },
      { x: 5, y: 2, rotate: 1.2, scale: 0.985 },
      { x: 10, y: 4, rotate: 2.4, scale: 0.97 },
    ];

    const expandedOffsets = [
      { x: -6, y: 0, rotate: -0.8 },
      { x: 8, y: -4, rotate: 1.1 },
      { x: 16, y: -8, rotate: 1.7 },
    ];

    const idx = Math.min(position, basePoses.length - 1);
    const base = basePoses[idx];
    const expand = expanded ? expandedOffsets[idx] : { x: 0, y: 0, rotate: 0 };

    if (!expanded) {
      return {
        x: 0,
        y: 0,
        rotate: 0,
        scale: 1,
      };
    }

    return {
      x: base.x + expand.x,
      y: base.y + expand.y,
      rotate: base.rotate + expand.rotate,
      scale: base.scale,
    };
  };

  const stackCards = getStackIndices(Math.min(3, images.length));
  const currentImage = images[currentIndex];

  return (
    <div
      className="relative w-[280px] h-[350px] sm:w-[400px] sm:h-[500px] cursor-pointer"
      onClick={handleClick}
      onMouseEnter={handleMouseEnter}
      onMouseLeave={handleMouseLeave}
    >
      <div
        className={`pointer-events-none absolute -inset-3 rounded-xl bg-black/70 blur-2xl transition-all duration-300 ease-out ${
          showFan ? 'opacity-100 scale-105' : 'opacity-0 scale-100'
        }`}
        style={{ zIndex: 0 }}
      />

      {stackCards.map((imageIndex, stackPosition) => {
        const pose = getStackPose(stackPosition, showFan);
        const image = images[imageIndex];
        const isTopCard = stackPosition === 0;

        return (
          <motion.div
            key={image.src}
            className={`absolute inset-0 rounded-lg overflow-hidden border ${
              showFan ? 'border-white/20' : 'border-white/0'
            } ${isTopCard ? 'shadow-2xl' : 'shadow-xl'}`}
            style={{
              zIndex: 20 - stackPosition,
              boxShadow: isTopCard && showFan ? '0 28px 70px rgba(0, 0, 0, 0.65)' : undefined,
            }}
            initial={false}
            animate={{ ...pose, opacity: isTopCard || showFan ? 1 : 0 }}
            transition={{ duration: 0.28, ease: [0.22, 1, 0.36, 1] }}
          >
            <img
              src={image.src}
              alt={`${image.title} by ${image.artist}`}
              loading={isTopCard && currentIndex === 0 ? 'eager' : 'lazy'}
              decoding="async"
              className="w-full h-full object-cover select-none pointer-events-none"
            />
            {isTopCard ? <div className="absolute inset-0 bg-gradient-to-t from-black/60 via-transparent to-transparent" /> : null}
          </motion.div>
        );
      })}

      <AnimatePresence initial={false}>
        {leavingCards.map((card) => {
          const topPose = getStackPose(0, showFan);

          return (
            <motion.div
              key={card.id}
              className={`pointer-events-none absolute inset-0 rounded-lg overflow-hidden border ${
                showFan ? 'border-white/20' : 'border-white/0'
              } shadow-2xl`}
              style={{ zIndex: 30 }}
              initial={{ ...topPose, opacity: 1 }}
              animate={{ x: topPose.x - 36, y: topPose.y - 2, rotate: topPose.rotate - 7, scale: 0.985, opacity: 0 }}
              transition={{ duration: 0.28, ease: [0.22, 1, 0.36, 1] }}
              onAnimationComplete={() =>
                setLeavingCards((prev) => prev.filter((prevCard) => prevCard.id !== card.id))
              }
            >
              <img
                src={card.image.src}
                alt={`${card.image.title} by ${card.image.artist}`}
                loading="lazy"
                decoding="async"
                className="w-full h-full object-cover select-none pointer-events-none"
              />
              <div className="absolute inset-0 bg-gradient-to-t from-black/60 via-transparent to-transparent" />
            </motion.div>
          );
        })}
      </AnimatePresence>

      <div
        className={`pointer-events-none absolute left-0 right-0 top-full mt-3 text-center transition-all duration-200 ${
          showFan ? 'opacity-100 translate-y-0' : 'opacity-0 -translate-y-1'
        }`}
        style={{ zIndex: 20 }}
      >
        <p className='text-sm font-medium text-white'>{currentImage.title}</p>
        <p className='text-xs text-zinc-400'>
          {currentImage.artist} · {currentImage.date}
        </p>
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
  thinkingImages: ThinkingImage[];
}

export const RedesignedHero = ({ latestChangelogTitle, thinkingImages }: RedesignedHeroProps) => {
  return (
    <section className='relative flex min-h-[100svh] flex-col justify-center px-6 pt-20 md:pt-0'>
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
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            transition={{ duration: 0.8, delay: 0.2 }}
            className='flex-shrink-0 hidden lg:block'
          >
            <ThinkingImageCycler images={thinkingImages} />
          </motion.div>
        </div>

        {/* Mobile: Image */}
        <div className='lg:hidden mt-12'>
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
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
