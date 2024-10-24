'use client';

import { Button } from '@rivet-gg/components';
import { motion } from 'framer-motion';
import { useState } from 'react';

export function Foldable({
  title = 'Show child properties',
  closeTitle = 'Hide child properties',
  children
}) {
  const [isOpen, setIsOpen] = useState(false);
  return (
    <>
      <Button variant='outline' size='sm' onClick={() => setIsOpen(open => !open)}>
        {isOpen ? closeTitle : title}
      </Button>
      <motion.div
        className='mt-1 overflow-hidden'
        initial={false}
        variants={{
          open: { height: 'auto', opacity: 1 },
          closed: { height: 0, opacity: 0 }
        }}
        animate={isOpen ? 'open' : 'closed'}
        transition={{
          opacity: isOpen ? { delay: 0.3 } : {},
          height: !isOpen ? { delay: 0.3 } : {},
          duration: 0.3
        }}>
        {children}
      </motion.div>
    </>
  );
}
