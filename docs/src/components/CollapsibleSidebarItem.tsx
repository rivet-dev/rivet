'use client';

import { SidebarItem, SidebarSection } from '@/lib/sitemap';
import { faChevronDown, Icon } from '@rivet-gg/icons';
import { motion } from 'framer-motion';
import { ReactNode, useState } from 'react';
import { usePathname } from 'next/navigation';

interface CollapsibleSidebarItemProps {
  item: SidebarSection;
  children?: ReactNode;
}

export function CollapsibleSidebarItem({ item, children }: CollapsibleSidebarItemProps) {
  const pathname = usePathname() || '';
  const isCurrent = findActiveItem(item.pages, pathname) !== null;
  const [isOpen, setIsOpen] = useState(() => isCurrent);
  return (
    <div>
      <button
        className='text-muted-foreground data-[active]:text-foreground flex w-full appearance-none items-center gap-4 px-2 py-1 text-sm transition-colors'
        data-active={isCurrent ? true : undefined}
        onClick={() => setIsOpen(open => !open)}>
        {item.title}
        <motion.span
          variants={{
            open: { rotateZ: 0 },
            closed: { rotateZ: '-90deg' }
          }}
          initial={isCurrent ? 'open' : 'closed'}
          animate={isOpen ? 'open' : 'closed'}
          className='-ml-2 mr-2 inline-block w-2.5'>
          <Icon icon={faChevronDown} className='size-auto' />
        </motion.span>
      </button>
      <motion.div
        className='overflow-hidden pl-1'
        initial={isCurrent ? 'open' : 'closed'}
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
    </div>
  );
}

function findActiveItem(pages: SidebarItem[] = [], href: string) {
  for (const page of pages) {
    if ('href' in page && page.href === href) {
      return page;
    }
    if ('pages' in page) {
      const found = findActiveItem(page.pages, href);
      if (found) {
        return found;
      }
    }
  }

  return null;
}
