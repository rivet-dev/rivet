'use client';
import Image from 'next/image';
import Link from 'next/link';
import { usePathname } from 'next/navigation';
import { Button } from '@/components/Button';
import clsx from 'clsx';
import routes from '@/generated/routes.json';

import imgLogo from '@/images/rivet-logos/icon-white.svg';
import { Icon, faDiscord, faGithub, faLinkedin, faTwitter, faBluesky, faYoutube } from '@rivet-gg/icons';

const footer = {
  docs: [
    { name: 'Documentation', href: '/docs' },
  ],
  company: [
    // { name: 'We\'re hiring!', href: 'https://rivet-gg.notion.site/Job-Board-eed66f2eab2b4d7ea3e21ccd63b22efe?pvs=4', newTab: true, highlight: true, badge: '1' },
    { name: 'Support', href: '/support' },
    { name: 'Pricing', href: '/pricing' },
    { name: 'Sales', href: '/sales' },
    { name: 'Status Page', href: 'https://rivet.betteruptime.com/' },
    { name: 'OSS Friends', href: '/oss-friends' }
  ],
  legal: [
    { name: 'Terms', href: '/terms' },
    { name: 'Privacy Policy', href: '/privacy' },
    { name: 'Acceptable Use', href: '/acceptable-use' }
  ],
  social: [
    {
      name: 'Discord',
      href: 'https://discord.gg/aXYfyNxYVn',
      icon: faDiscord
    },
    {
      name: 'Twitter',
      href: 'https://twitter.com/rivet_gg',
      icon: faTwitter
    },
    {
      name: 'Bluesky',
      href: 'https://bsky.app/profile/rivet.gg',
      icon: faBluesky
    },
    {
      name: 'GitHub',
      href: 'https://github.com/rivet-gg',
      icon: faGithub
    },
    {
      name: 'YouTube',
      href: 'https://www.youtube.com/@rivet-gg',
      icon: faYoutube
    },
    {
      name: 'LinkedIn',
      href: 'https://www.linkedin.com/company/72072261/',
      icon: faLinkedin
    }
  ]
};

function PageLink({ label, page, previous = false }) {
  let title = routes.pages[page.href]?.title ?? page.title ?? label;
  return (
    <>
      <Button
        href={page.href}
        aria-label={`${label}: ${page.title}`}
        variant='secondary'
        arrow={previous ? 'left' : 'right'}>
        {title}
      </Button>
    </>
  );
}

export function PageNextPrevious({ navigation }) {
  let pathname = usePathname();
  let allPages = navigation.sidebar.groups.flatMap(group => group.pages);
  let currentPageIndex = allPages.findIndex(page => page.href === pathname);

  if (currentPageIndex === -1) {
    return null;
  }

  let previousPage = allPages[currentPageIndex - 1];
  let nextPage = allPages[currentPageIndex + 1];

  if (!previousPage && !nextPage) {
    return null;
  }

  return (
    <div className={clsx('mb-4 flex', 'mx-auto max-w-5xl')}>
      {previousPage && (
        <div className='flex flex-col items-start gap-3'>
          <PageLink label='Previous' page={previousPage} previous />
        </div>
      )}
      {nextPage && (
        <div className='ml-auto flex flex-col items-end gap-3'>
          <PageLink label='Next' page={nextPage} />
        </div>
      )}
    </div>
  );
}

function SmallPrint() {
  return (
    <div className='mx-auto max-w-screen-2xl pb-8 pt-16 sm:pt-20'>
      <div className='xl:grid xl:grid-cols-2 xl:gap-8'>
        {/* Brands & links */}
        <div className='space-y-8'>
          {/* Logo */}
          <Image className='size-12' src={imgLogo} alt='Rivet' />
          <p className='text-sm leading-6 text-gray-300'>
            Run and scale realtime applications
          </p>

          {/* Social */}
          <div className='flex space-x-6'>
            {footer.social.map(item => (
              <Link key={item.name} href={item.href} className='text-xl text-gray-500 hover:text-gray-400'>
                <span className='sr-only'>{item.name}</span>
                <Icon icon={item.icon} aria-hidden='true' />
              </Link>
            ))}
          </div>
        </div>

        {/* Links */}
        <div className='mt-16 grid grid-cols-1 gap-8 md:grid-cols-3 xl:mt-0'>
          <div>
            <div className='text-sm font-semibold leading-6 text-white'>Documentation</div>
            <ul role='list' className='mt-3 space-y-2'>
              {footer.docs.map(item => (
                <li key={item.name}>
                  <Link
                    href={item.href}
                    target={item.target}
                    className='text-sm leading-4 text-gray-300 hover:text-white'>
                    {item.name}
                  </Link>
                </li>
              ))}
            </ul>
          </div>
          <div>
            <div className='text-sm font-semibold leading-6 text-white'>Company</div>
            <ul role='list' className='mt-3 space-y-2'>
              {footer.company.map(item => (
                <li key={item.name}>
                  <Link
                    href={item.href}
                    target={item.newTab ? '_blank' : null}
                    className={clsx('text-sm leading-4 text-gray-300 hover:text-white')}>
                    <span
                      className={clsx(
                        item.highlight && 'text-violet-200 drop-shadow-[0_0_10px_rgba(221,214,254,0.5)]'
                      )}>
                      {item.name}
                    </span>
                    {item.badge && <span className='ml-2 rounded-full bg-violet-500 px-2'>{item.badge}</span>}
                  </Link>
                </li>
              ))}
            </ul>
          </div>
          <div className='mt-10 md:mt-0'>
            <div className='text-sm font-semibold leading-6 text-white'>Legal</div>
            <ul role='list' className='mt-3 space-y-2'>
              {footer.legal.map(item => (
                <li key={item.name}>
                  <Link href={item.href} className='text-sm leading-4 text-gray-300 hover:text-white'>
                    {item.name}
                  </Link>
                </li>
              ))}
            </ul>
          </div>
        </div>
      </div>

      {/* Footer */}
      <div className='mt-4 border-t border-white/10 pt-4 text-center md:mt-8'>
        <p className='text-xs leading-5 text-white'>
          &copy; {new Date().getFullYear()} Rivet Gaming, Inc. All rights reserved.
        </p>
      </div>
    </div>
  );
}

export function Footer() {
  return (
    <div>
      <hr className='mb-8 border-white/10' />

      <footer aria-labelledby='footer-heading' className='mx-auto max-w-screen-xl px-6 lg:px-8'>
        <h2 id='footer-heading' className='sr-only'>
          Footer
        </h2>
        <SmallPrint />
      </footer>
    </div>
  );
}
