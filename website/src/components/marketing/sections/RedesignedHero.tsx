'use client';

import { useEffect, useRef, useState } from 'react';
import { Terminal, ArrowRight, Check } from 'lucide-react';
import { AnimatePresence, motion, useScroll, useTransform } from 'framer-motion';
import { HERO_H1_CLASS } from '../typography';
import { GLOW_PILL_CLASS, handleGlowPillMouseMove } from '../glowPill';

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
    const indices: number[] = [];
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
      { x: -8, y: 0, rotate: -0.9 },
      { x: 10, y: -5, rotate: 1.3 },
      { x: 20, y: -10, rotate: 2 },
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
      {/* Soft ground beneath the stack when fanned */}
      <div
        className={`pointer-events-none absolute -inset-3 rounded-xl bg-ink/5 blur-xl transition-all duration-300 ease-out ${
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
            className="absolute inset-0 overflow-hidden border border-ink/10"
            style={{ zIndex: 20 - stackPosition, boxShadow: '0 10px 28px -12px rgba(27, 25, 22, 0.16)' }}
            initial={false}
            animate={{ ...pose, opacity: isTopCard || showFan ? 1 : 0 }}
            transition={{ duration: 0.28, ease: [0.22, 1, 0.36, 1] }}
          >
            <img
              src={image.src}
              alt={`${image.title} by ${image.artist}`}
              loading={isTopCard && currentIndex === 0 ? 'eager' : 'lazy'}
              decoding="async"
              className="h-full w-full select-none object-cover pointer-events-none"
            />
          </motion.div>
        );
      })}

      <AnimatePresence initial={false}>
        {leavingCards.map((card) => {
          const topPose = getStackPose(0, showFan);

          return (
            <motion.div
              key={card.id}
              className="pointer-events-none absolute inset-0 overflow-hidden border border-ink/10"
              style={{ zIndex: 30, boxShadow: '0 10px 28px -12px rgba(27, 25, 22, 0.16)' }}
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
                className="h-full w-full select-none object-cover pointer-events-none"
              />
            </motion.div>
          );
        })}
      </AnimatePresence>

      <div
        className={`pointer-events-none absolute left-0 right-0 top-full mt-5 text-center transition-all duration-200 ${
          showFan ? 'opacity-100 translate-y-0' : 'opacity-0 -translate-y-1'
        }`}
        style={{ zIndex: 20 }}
      >
        <AnimatePresence mode="wait" initial={false}>
          <motion.div
            key={currentImage.src}
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            transition={{ duration: 0.2 }}
          >
            <p className="text-sm font-medium leading-tight text-ink">{currentImage.title}</p>
            <p className="mt-1 text-xs leading-tight text-ink-faint">
              {currentImage.artist} · {currentImage.date}
            </p>
          </motion.div>
        </AnimatePresence>
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
        className='w-full sm:w-auto inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md border border-ink/20 px-4 py-2 font-mono text-[13px] text-ink-soft transition-colors hover:border-ink/40 hover:text-ink'
      >
        {copied ? <Check className='h-4 w-4 text-pine' /> : <Terminal className='h-4 w-4' />}
        npx skills add rivet-dev/skills
      </button>
      <div className='absolute left-1/2 -translate-x-1/2 top-full mt-4 opacity-0 translate-y-2 group-hover:opacity-100 group-hover:translate-y-0 transition-all duration-200 ease-out text-xs text-ink-faint whitespace-nowrap pointer-events-none font-mono'>
        Give this to your coding agent
      </div>
    </div>
  );
};

interface RedesignedHeroProps {
  latestChangelogTitle: string;
  latestChangelogHref: string;
  thinkingImages: ThinkingImage[];
}

export const RedesignedHero = ({ latestChangelogTitle, latestChangelogHref, thinkingImages }: RedesignedHeroProps) => {
  const heroRef = useRef<HTMLElement>(null);
  // Fade the hero out as it scrolls away. Anchored to the hero's own height
  // (start..end against the top of the viewport) rather than a fixed viewport
  // fraction, so it behaves on mobile where the stacked text + image make the
  // hero taller than the screen. Opacity is the only transform, so it stays
  // calm under prefers-reduced-motion.
  const { scrollYProgress } = useScroll({
    target: heroRef,
    offset: ['start start', 'end start'],
  });
  const heroOpacity = useTransform(scrollYProgress, [0, 0.55], [1, 0]);

  return (
    <motion.section
      ref={heroRef}
      style={{ opacity: heroOpacity }}
      className='depth-wash relative flex min-h-[100svh] flex-col justify-center overflow-hidden px-6 pt-32 pb-16 will-change-[opacity] md:pt-40 md:pb-24'
    >
      <div className='relative mx-auto w-full max-w-7xl'>
        <div className='flex flex-col gap-12 lg:flex-row lg:items-center lg:justify-between lg:gap-32 xl:gap-48 2xl:gap-64'>
          <div className='max-w-xl'>
            <motion.div
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              transition={{ duration: 0.5, delay: 0.15 }}
              className='mb-7'
            >
              <a
                href={latestChangelogHref}
                className={`${GLOW_PILL_CLASS} group inline-flex items-center gap-2 rounded-full border border-ink/12 bg-paper/45 px-2.5 py-1 text-[13px] text-ink-soft shadow-[0_8px_22px_-20px_rgba(27,25,22,0.45)] transition-colors hover:border-ink/25 hover:text-ink`}
                onMouseMove={handleGlowPillMouseMove}
              >
                <span
                  aria-hidden='true'
                  className='h-1 w-1 rounded-full bg-accent'
                  style={{
                    boxShadow:
                      '0 0 2px rgba(203, 90, 51, 0.9), 0 0 6px rgba(203, 90, 51, 0.5), 0 0 14px rgba(171, 69, 31, 0.35)',
                  }}
                />
                <span>{latestChangelogTitle}</span>
                <ArrowRight className='h-3 w-3 text-ink-soft transition-transform group-hover:translate-x-0.5' />
              </a>
            </motion.div>

            <motion.h1
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ duration: 0.5 }}
              className={`mb-5 ${HERO_H1_CLASS}`}
            >
              Infrastructure for <br />
              software that thinks.
            </motion.h1>

            <motion.p
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ duration: 0.5, delay: 0.05 }}
              className='mb-8 max-w-xl text-lg leading-relaxed text-ink-soft'
            >
              Actors are the primitive for AI agents — and the realtime apps and workflows around them. They burst when traffic spikes, sleep when idle, and run anywhere: Rivet Cloud, your VPC, or fully air-gapped.
            </motion.p>

            <motion.div
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ duration: 0.5, delay: 0.1 }}
              className='flex flex-col gap-3 sm:flex-row'
            >
              <a href='/docs'
                className='inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md bg-accent-deep px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-accent'
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
            className='relative flex-shrink-0 hidden lg:block'
          >
            <ThinkingImageCycler images={thinkingImages} />
          </motion.div>
        </div>

        {/* Mobile: Image */}
        <div className='lg:hidden mt-12 mb-10'>
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
    </motion.section>
  );
};
