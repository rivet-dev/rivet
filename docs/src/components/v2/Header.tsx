import Link from 'next/link';
import Image from 'next/image';
import logoUrl from '@/images/rivet-logos/icon-text-white.svg';
import { Header as RivetHeader } from '@rivet-gg/components/header';
import { Button, cn } from '@rivet-gg/components';
import { ReactNode } from 'react';
import { DocsMobileNavigation } from '@/components/DocsMobileNavigation';
import { faDiscord, faGithub, Icon } from '@rivet-gg/icons';

interface HeaderProps {
  active?: 'product' | 'docs' | 'blog' | 'pricing';
  subnav?: ReactNode;
  mobileBreadcrumbs?: ReactNode;
}

export function Header({ active, subnav }: HeaderProps) {
  const headerStyles = cn({
    'md:border-3  md:border-transparent md:bg-white/5 md:fixed md:left-1/2 md:w-full md:top-4 md:max-w-[1200px] md:-translate-x-1/2  md:rounded-2xl md:border-transparent md:backdrop-blur-xl':
      !active
  });
  return (
    <RivetHeader
      className={headerStyles}
      logo={
        <Link href='/'>
          <Image {...logoUrl} className='ml-1 w-20' alt='Rivet logo' />
        </Link>
      }
      subnav={subnav}
      support={
        <div className='flex flex-col gap-4 font-v2 subpixel-antialiased'>
          <RivetHeader.NavItem asChild>
            <Link href='https://hub.rivet.gg'>Sign In</Link>
          </RivetHeader.NavItem>
          <RivetHeader.NavItem asChild>
            <Link href='/discord'>Discord</Link>
          </RivetHeader.NavItem>
          <RivetHeader.NavItem asChild>
            <Link href='/support'>Support</Link>
          </RivetHeader.NavItem>
        </div>
      }
      links={
        <>
          <RivetHeader.NavItem asChild className='-m-2 p-2'>
            <Link href='/discord' className='text-white/50'>
              <Icon icon={faDiscord} />
            </Link>
          </RivetHeader.NavItem>
          <RivetHeader.NavItem asChild className='p-2'>
            <Link href='/support' className='text-white/50'>
              <Icon icon={faGithub} />
            </Link>
          </RivetHeader.NavItem>
          <Button variant='secondary' asChild className='font-v2 subpixel-antialiased'>
            <Link href='https://hub.rivet.gg'>Sign In</Link>
          </Button>
        </>
      }
      mobileBreadcrumbs={<DocsMobileNavigation />}
      breadcrumbs={
        <div className='flex items-center gap-5 font-v2 subpixel-antialiased'>
          <RivetHeader.NavItem asChild className='flex items-center gap-1 py-2'>
            <Link href='/docs' aria-current={active === 'product' ? 'page' : undefined}>
              Product
            </Link>
          </RivetHeader.NavItem>
          <RivetHeader.NavItem asChild className='flex items-center gap-1 py-2'>
            <Link href='/docs' aria-current={active === 'docs' ? 'page' : undefined}>
              Docs
            </Link>
          </RivetHeader.NavItem>
          <RivetHeader.NavItem asChild className='flex items-center gap-1'>
            <Link href='/changelog' aria-current={active === 'blog' ? 'page' : undefined}>
              Changelog
            </Link>
          </RivetHeader.NavItem>
          <RivetHeader.NavItem asChild className='flex items-center gap-1'>
            <Link href='/pricing' aria-current={active === 'pricing' ? 'page' : undefined}>
              Pricing
            </Link>
          </RivetHeader.NavItem>
        </div>
      }
    />
  );
}
