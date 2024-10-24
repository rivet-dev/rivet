import { AnimatePresence, motion } from 'framer-motion';
import clsx from 'clsx';

import { Footer, PageNextPrevious } from '@/components/Footer';
import { Navigation } from '@/components/Navigation';
import { TableOfContents } from '@/components/TableOfContents';
import { Prose } from '@/components/Prose';
import { HeroPattern } from '@/components/HeroPattern';
import { Feedback } from '@/components/Feedback';
import { Header } from '@/components/v2/Header';

export function Layout({
  navigation,
  isTopPage,
  fullWidth,
  tableOfContents,
  prose,
  inset,
  children,
  sections = [],
  pathname
}) {
  console.log(pathname);
  return (
    <div>
      <Header active={pathname === '/pricing' ? 'pricing' : ''} />

      {/* Body */}
      {/* <div className={clsx('relative', navigation.tabs ? 'pt-navigation' : 'pt-14')}> */}
      <div className={clsx('relative')}>
        {(prose || inset) && isTopPage ? <HeroPattern /> : null}

        <div
          className={clsx(
            { 'w-full': prose || inset, 'main-content-container w-full px-6': prose },
            'flex w-full flex-col-reverse lg:flex-row'
          )}>
          {navigation.sidebar ? (
            <aside
              className={clsx(
                `lg:top-navigation hidden w-full lg:pointer-events-auto lg:sticky lg:max-h-tabs-content lg:min-h-tabs-content lg:max-w-aside lg:self-start lg:overflow-y-auto lg:border-r lg:border-charcole-900/10 lg:pb-8 lg:pe-6 lg:pt-4 lg:dark:border-white/10 xl:block`
              )}>
              <Navigation navigation={navigation} />
            </aside>
          ) : null}

          <main
            className={clsx(
              {
                'lg:px-8': navigation.sidebar,
                'lg:max-w-3xl': !fullWidth
              },
              'mx-auto mt-9 w-full flex-1'
            )}>
            {prose ? <Prose as='article'>{children}</Prose> : children}

            {navigation.feedback || navigation.sidebar ? (
              <div className='mb-4 mt-20'>
                {navigation.sidebar ? <PageNextPrevious navigation={navigation} /> : null}
                {navigation.feedback ? <Feedback /> : null}
              </div>
            ) : null}
          </main>

          {/* Table of contents */}
          <TableOfContents />
        </div>

        <Footer />
      </div>
    </div>
  );
}
