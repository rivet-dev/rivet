'use client';

import Link from 'next/link';
import { useState, useCallback, useRef } from 'react';
import clsx from 'clsx';
import { motion } from 'framer-motion';
import { useEffect } from 'react';
import { remToPx } from '@/lib/remToPx';
import { useNavigation } from '@/hooks/useNavigation';

const HEADER_HEIGHT = remToPx(6.5);
// const SCROLL_MARGIN = remToPx(9 /* scroll-mt-header-offset */ - HEADER_HEIGHT);
const LINK_MARGIN = remToPx(1);

function useScrollToActiveLink(currentSection) {
  let ref = useRef(null);

  useEffect(() => {
    let currentLink = ref.current?.querySelector(`[aria-current="page"]`);
    if (!currentLink) return;

    let linkRect = currentLink?.getBoundingClientRect();
    let containerRect = ref.current?.getBoundingClientRect();

    // calculate how much to scroll by
    // take into account the navigation header height
    let linkRelativeTop = linkRect.y - containerRect.top;

    // if the link is below the container, scroll down by the difference in height + the height of the link itself (so it's not at the bottom)
    if (linkRelativeTop + LINK_MARGIN >= containerRect.height) {
      // calculate the difference between the bottom of the link and the bottom of the container
      let bottomDifference = linkRelativeTop + LINK_MARGIN - containerRect.height + linkRect.height;
      ref.current.scrollBy(0, bottomDifference);
    }
    // if the link is above the container, scroll up by the difference in height + the height of the link itself (so it's not at the top)
    else if (linkRelativeTop < linkRect.height + LINK_MARGIN) {
      ref.current.scrollBy(0, linkRelativeTop - LINK_MARGIN);
    }
  }, [currentSection]);

  return ref;
}

function useCurrentSection(tableOfContents = []) {
  let [currentSection, setCurrentSection] = useState(tableOfContents?.[0]?.id || null);
  let getHeadings = useCallback(tableOfContents => {
    return tableOfContents
      .flatMap(node => [node.id, ...node.children.map(child => child.id)])
      .map(id => {
        let el = document.getElementById(id);
        if (!el) return null;

        let style = window.getComputedStyle(el);
        let scrollMt = parseFloat(style.scrollMarginTop);

        let top = window.scrollY + el.getBoundingClientRect().top - scrollMt;
        return { id, top };
      })
      .filter(x => x !== null);
  }, []);

  useEffect(() => {
    if (!tableOfContents || tableOfContents?.length === 0) return;
    let headings = getHeadings(tableOfContents);
    if (headings.length === 0) return;
    function onScroll() {
      let top = window.scrollY;
      let current = headings[0].id;
      for (let heading of headings) {
        if (top >= heading.top - LINK_MARGIN) {
          current = heading.id;
        } else {
          break;
        }
      }
      setCurrentSection(current);
    }
    window.addEventListener('scroll', onScroll, { passive: true });
    onScroll();
    return () => {
      window.removeEventListener('scroll', onScroll);
    };
  }, [getHeadings, tableOfContents]);
  return currentSection;
}

function NavLink({ id, isActive, isAnchorLink = false, children }) {
  return (
    <>
      <Link
        href={`#${id}`}
        aria-current={isActive ? 'page' : undefined}
        className={clsx(
          'flex justify-between gap-2 py-1 pr-3 text-sm transition',
          isAnchorLink ? 'pl-7' : 'pl-4',
          isActive ? 'text-white' : 'text-charcole-400 hover:text-white'
        )}>
        <span className='truncate'>{children}</span>
      </Link>
    </>
  );
}

export function ActiveSectionMarker({ prefix }) {
  return (
    <>
      <motion.div
        layout
        layoutId={`${prefix}current-background`}
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        className='absolute inset-0 -left-2 bg-charcole-800/2.5 will-change-transform dark:bg-white/2.5'
        style={{ borderRadius: 8 }}
      />
      <motion.div
        layout
        layoutId={`${prefix}current-line`}
        className='absolute left-0 top-1 h-6 w-px bg-cream-500'
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
      />
    </>
  );
}

function Tree({ sections, isActive, depth = 0 }) {
  return (
    <>
      <motion.div layout className='absolute inset-y-0 left-2 w-px bg-charcole-950/10 dark:bg-white/5' />
      <ul role='list'>
        {sections.map(section => {
          const isCurrentSectionActive = isActive(section);
          return (
            <li key={section.id} className='relative'>
              <div className='relative'>
                {isCurrentSectionActive ? <ActiveSectionMarker /> : null}
                <NavLink key={section.id} id={section.id} isAnchorLink isActive={isCurrentSectionActive}>
                  {section.title}
                </NavLink>
              </div>

              {section.children.length > 0 ? (
                <div className='relative ml-5 mt-1 pl-2'>
                  <Tree sections={section.children} isActive={isActive} depth={depth + 1} />
                </div>
              ) : null}
            </li>
          );
        })}
      </ul>
    </>
  );
}

export function TableOfContents({ tableOfContents: providedToc }) {
  let { tableOfContents: navigationToc } = useNavigation();

  let tableOfContents = providedToc ?? navigationToc;

  let currentSection = useCurrentSection(tableOfContents);
  let ref = useScrollToActiveLink(currentSection);

  if (!tableOfContents) {
    return null;
  }

  function isActive(section) {
    if (section.id === currentSection) {
      return true;
    }
    if (!section.children) {
      return false;
    }
  }

  return (
    <div
      ref={ref}
      className={clsx(
        'pt-4 lg:top-docs-navigation lg:max-h-tabs-content lg:pt-6',
        'w-full lg:pointer-events-auto lg:sticky lg:block lg:max-w-aside lg:self-start lg:overflow-y-auto'
      )}>
      <div className='relative'>
        <motion.h2 layout='position' className='font-sans text-xs font-semibold text-white'>
          On this page
        </motion.h2>

        <div className='relative mt-3 pl-2'>
          <Tree sections={tableOfContents} isActive={isActive} />
        </div>
      </div>
    </div>
  );
}
