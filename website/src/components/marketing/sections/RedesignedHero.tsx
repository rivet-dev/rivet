'use client';

import { useEffect, useRef, useState } from 'react';
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

// Animation timing (ms)
const TM = {
  FADE: 300,
  R_START: 500,
  R_LOAD: 400,
  R_ACTION: 400,
  R_GAP: 200,
  R_COUNT: 3,
  A_DELAY: 600,
  A_LOAD: 400,
  A_ACTION: 167,
  A_COUNT: 3,
  PAUSE: 2500,
};

const R_CYCLE = TM.R_LOAD + TM.R_ACTION + TM.R_GAP;
const R_END = TM.R_START + R_CYCLE * TM.R_COUNT;
const A_START = R_END + TM.A_DELAY;
const A_LOAD_END = A_START + TM.A_LOAD;
const A_END = A_LOAD_END + TM.A_ACTION * TM.A_COUNT;
const CYCLE_TOTAL = A_END + TM.PAUSE;

function bp(now: number, start: number, dur: number) {
  if (now < start) return 0;
  if (now >= start + dur) return 1;
  return (now - start) / dur;
}

const BW = 40;
const BH = 36;
const BG = 4;
const XW = 20;
const GG = 20;
const GS = BW + BG + BW + BG + XW + GG;

const ArchitectureGraphic = () => {
  const [t, setT] = useState(0);
  const originRef = useRef(0);

  useEffect(() => {
    let raf: number;
    originRef.current = performance.now();
    const tick = (now: number) => {
      const elapsed = now - originRef.current;
      if (elapsed >= CYCLE_TOTAL) {
        originRef.current = now;
        setT(0);
      } else {
        setT(elapsed);
      }
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, []);

  const fade = Math.min(1, t / TM.FADE);
  const rActive = t >= TM.R_START && t < R_END;
  const aActive = t >= A_START && t < A_END;
  const done = t >= A_END;

  const rTime = t < TM.R_START ? 0 : t >= R_END ? (R_END - TM.R_START) / 1000 : (t - TM.R_START) / 1000;
  const aTime = t < A_START ? 0 : t >= A_END ? (A_END - A_START) / 1000 : (t - A_START) / 1000;
  const rTimeFinal = (R_END - TM.R_START) / 1000;
  const aTimeFinal = (A_END - A_START) / 1000;

  return (
    <div className="w-full">
      <svg viewBox="0 0 560 280" className="w-full h-auto" xmlns="http://www.w3.org/2000/svg" style={{ opacity: fade }}>
        {/* Time axis */}
        <text x="28" y="14" fill="#52525b" fontSize={9} fontWeight={500} letterSpacing={2}>TIME →</text>
        <line x1="80" y1="11" x2="480" y2="11" stroke="#27272a" strokeWidth={0.5} />

        {/* Top: Request/Response */}
        <g style={{ opacity: aActive ? 0.3 : 1, transition: 'opacity 0.4s ease' }}>
          <rect x="12" y="24" width="536" height="106" rx="6"
            fill={rActive ? 'rgba(255,96,48,0.04)' : 'transparent'}
            stroke={rActive ? '#ff6030' : '#27272a'}
            strokeWidth={rActive ? 1.5 : 0.5}
            style={{ transition: 'all 0.3s ease' }} />
          <text x="28" y="44" fill="#d4d4d8" fontSize={11} fontWeight={600} letterSpacing={0.5}>REQUEST / RESPONSE</text>

          {Array.from({ length: TM.R_COUNT }, (_, i) => {
            const gx = 28 + i * GS;
            const rs = TM.R_START + i * R_CYCLE;
            return (
              <g key={i}>
                <rect x={gx} y={58} width={BW * bp(t, rs, TM.R_LOAD)} height={BH} rx={3} fill="#ff6030" />
                <rect x={gx + BW + BG} y={58} width={BW * bp(t, rs + TM.R_LOAD, TM.R_ACTION)} height={BH} rx={3} fill="#30A46C" />
                {t >= rs + TM.R_LOAD + TM.R_ACTION && (
                  <text x={gx + BW + BG + BW + XW / 2} y={81} textAnchor="middle" fill="#ef4444" fontSize={14} fontWeight={700} opacity={0.7}>×</text>
                )}
                <text x={gx + (BW * 2 + BG) / 2} y={110} textAnchor="middle" fill="#52525b" fontSize={9}>req {i + 1}</text>
              </g>
            );
          })}

          <text x="524" y="80" textAnchor="end"
            fill={rActive ? '#ff6030' : t >= R_END ? '#a1a1aa' : '#3f3f46'}
            fontSize={18} fontWeight={700} fontFamily="ui-monospace, monospace"
            style={{ transition: 'fill 0.3s ease' }}>
            {rTime.toFixed(1)}s
          </text>
        </g>

        {/* Divider */}
        <line x1="28" y1="140" x2="532" y2="140" stroke="#27272a" strokeWidth={0.5} />

        {/* Bottom: Rivet Actors */}
        <g style={{ opacity: rActive ? 0.3 : 1, transition: 'opacity 0.4s ease' }}>
          <rect x="12" y="148" width="536" height="106" rx="6"
            fill={aActive ? 'rgba(48,164,108,0.04)' : 'transparent'}
            stroke={aActive ? '#30A46C' : '#27272a'}
            strokeWidth={aActive ? 1.5 : 0.5}
            style={{ transition: 'all 0.3s ease' }} />
          <text x="28" y="168" fill="#d4d4d8" fontSize={11} fontWeight={600} letterSpacing={0.5}>RIVET ACTORS</text>

          <rect x={28} y={182} width={BW * bp(t, A_START, TM.A_LOAD)} height={BH} rx={3} fill="#ff6030" />
          {Array.from({ length: TM.A_COUNT }, (_, j) => (
            <rect key={j} x={28 + BW + BG + j * (BW + BG)} y={182}
              width={BW * bp(t, A_LOAD_END + j * TM.A_ACTION, TM.A_ACTION)} height={BH} rx={3} fill="#30A46C" />
          ))}

          <text x={28 + BW / 2} y={234} textAnchor="middle" fill="#52525b" fontSize={9}>load</text>
          {t >= A_LOAD_END + TM.A_ACTION && (
            <text x={28 + BW + BG + (TM.A_COUNT * (BW + BG) - BG) / 2} y={234}
              textAnchor="middle" fill="#52525b" fontSize={9}>req 1 · req 2 · req 3</text>
          )}

          <text x="524" y="204" textAnchor="end"
            fill={aActive ? '#30A46C' : t >= A_END ? '#a1a1aa' : '#3f3f46'}
            fontSize={18} fontWeight={700} fontFamily="ui-monospace, monospace"
            style={{ transition: 'fill 0.3s ease' }}>
            {aTime.toFixed(1)}s
          </text>

          {done && (
            <text x="524" y="222" textAnchor="end" fill="#30A46C" fontSize={11} fontWeight={600}
              opacity={Math.min(1, (t - A_END) / 400)}>
              {(rTimeFinal / aTimeFinal).toFixed(1)}x faster
            </text>
          )}
        </g>

        {/* Legend */}
        <g opacity={0.8}>
          <rect x="28" y="264" width="8" height="8" rx="2" fill="#ff6030" />
          <text x="42" y="272" fill="#52525b" fontSize={9}>load context</text>
          <rect x="120" y="264" width="8" height="8" rx="2" fill="#30A46C" />
          <text x="134" y="272" fill="#52525b" fontSize={9}>perform action</text>
          <text x="222" y="273" fill="#ef4444" fontSize={12} fontWeight={700}>×</text>
          <text x="234" y="272" fill="#52525b" fontSize={9}>state lost</text>
        </g>
      </svg>
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
        <div className='flex flex-col gap-12 lg:flex-row lg:items-center lg:gap-16 xl:gap-20'>
          <div className='max-w-md lg:shrink-0'>
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
              className='mb-4 text-2xl font-normal leading-[1.2] tracking-tight text-white md:text-[2rem]'
            >
              The web was built for<br />
              request/response.<br />
              <span className="text-zinc-500">AI broke that architecture.</span>
            </motion.h1>

            <motion.p
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ duration: 0.5, delay: 0.05 }}
              className='mb-6 text-lg text-zinc-400 md:text-xl'
            >
              Rivet Actors are a serverless primitive for stateful backends.
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
            transition={{ duration: 0.6, delay: 0.2 }}
            className='hidden lg:block min-w-0 flex-1'
          >
            {/* <ThinkingImageCycler images={thinkingImages} /> */}
            <ArchitectureGraphic />
          </motion.div>
        </div>

        {/* Mobile: Graphic */}
        <div className='lg:hidden mt-12'>
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            transition={{ duration: 0.6, delay: 0.2 }}
            className='flex justify-center'
          >
            {/* <ThinkingImageCycler images={thinkingImages} /> */}
            <ArchitectureGraphic />
          </motion.div>
        </div>
      </div>
    </section>
  );
};
