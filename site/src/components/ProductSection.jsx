import React, { useState } from 'react';
import Image from 'next/image';
import { Button } from '@/components/Button';
import { motion, AnimatePresence } from 'framer-motion';

// Import your new images here
import imgGameServerManagement from '@/images/product/game-server-management.png';
import imgBackendEditor from '@/images/product/backend-editor.png';
import imgBackendLogs from '@/images/product/backend-logs.png';
import imgVersionsRollback from '@/images/product/version-management.png';

const PRODUCT_PAGES = [
  {
    name: 'Game Server Management',
    description: 'Manage running servers, see players online, view logs, inspect crashes, and monitor performance.',
    image: imgGameServerManagement,
  },
  {
    name: 'Versions & Rollback',
    description: 'View version history & roll back instantly without re-deploying.',
    image: imgVersionsRollback,
  },
  {
    name: 'Backend Editor',
    description: 'Add, configure, & develop modules visually. No more confusing config files & reading extensive documentation.',
    image: imgBackendEditor,
  },
  {
    name: 'Backend Logs',
    description: 'Remove the guesswork from running your game with full visibility to everything happening on the backend.',
    image: imgBackendLogs,
  }
];

export default function ProductSection() {
  const [page, setPage] = useState({ index: 0, dir: 1 });

  const changePage = i => setPage({ index: i, dir: i > page.index ? 1 : -1 });

  return (
    <div className='flex flex-col items-center gap-16 px-4'>
      <h2 className='text-center font-display text-5xl font-extrabold tracking-tight text-cream-100 sm:text-5xl'>
        A Single Platform to Manage Your Game Servers & Backend
      </h2>

      <div className='flex w-full flex-col items-stretch gap-6'>
        {/* Product tabs */}
        <div className='flex flex-wrap justify-center gap-2'>
          {PRODUCT_PAGES.map((product, i) => (
            <Button key={i} variant='juicy' highlight={i == page.index} onMouseEnter={() => changePage(i)}>
              {product.name}
            </Button>
          ))}
        </div>

        {/* Current product */}
        <ProductPages page={page} onChangePage={setPage} />
      </div>
    </div>
  );
}

function ProductPages({ page, onChangePage }) {
  return (
    <div className='relative h-[500px] md:h-[800px]'>
      <AnimatePresence initial={false} custom={page.dir}>
        <motion.div
          key={page.index}
          className='absolute flex h-full w-full flex-col items-center gap-8'
          custom={page.dir}
          variants={variants}
          initial='enter'
          animate='center'
          exit='exit'
          transition={{
            x: { type: 'spring', stiffness: 300, damping: 30 },
            opacity: { duration: 0.2 }
          }}
          drag='x'
          dragConstraints={{ left: 0, right: 0 }}
          dragElastic={1}
          onDragEnd={(e, { offset, velocity }) => {
            const swipe = swipePower(offset.x, velocity.x);

            if (swipe < -swipeConfidenceThreshold) {
              onChangePage(paginate(page.index, 1, PRODUCT_PAGES));
            } else if (swipe > swipeConfidenceThreshold) {
              onChangePage(paginate(page.index, -1, PRODUCT_PAGES));
            }
          }}>
          <ProductPageContents page={PRODUCT_PAGES[page.index]} scale={page.index === 3} />
        </motion.div>
      </AnimatePresence>
    </div>
  );
}

function ProductPageContents({ page, scale }) {
  return (
    <>
      <p className='text-cream-100 text-center max-w-xl'>{page.description}</p>
      <div className='flex flex-grow items-center justify-center overflow-hidden'>
        <div className='flex h-full items-center justify-center'>
          <Image
            src={page.image}
            alt={`${page.name} Image`}
            className='max-h-full w-auto rounded border-2 border-cream-100/10 object-contain'
            width={500}
            height={300}
          />
        </div>
      </div>
    </>
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
