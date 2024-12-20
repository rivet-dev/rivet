'use client';
import Link from 'next/link';
import Image from 'next/image';
import logoUrl from '@/images/rivet-logos/icon-text-white.svg';
import { Header as RivetHeader } from '@rivet-gg/components/header';
import {
  Button,
  Tooltip,
  TooltipTrigger,
  TooltipContent,
  TooltipPortal,
  TooltipArrow,
  TooltipProvider
} from '@rivet-gg/components';
import { ReactNode, useState } from 'react';
import { DocsMobileNavigation } from '@/components/DocsMobileNavigation';
import { faDiscord, faGithub, Icon } from '@rivet-gg/icons';
import { HeaderPopupProductMenu } from '../HeaderPopupProductMenu';
import { HeaderPopupSolutionsMenu } from '../HeaderPopupSolutionsMenu';

interface HeaderProps {
  active: 'product' | 'docs' | 'blog' | 'pricing' | 'solutions';
  subnav?: ReactNode;
  mobileBreadcrumbs?: ReactNode;
}

export function Header({ active, subnav }: HeaderProps) {
  const [ref, setRef] = useState<Element | null>(null);
  return (
    <RivetHeader
      className='px-8 md:[&>div:first-child]:max-w-[calc(20rem+65ch+20rem)] md:[&>div:first-child]:px-0'
      logo={
        <Link href='/'>
          <Image {...logoUrl} className='w-20' alt='Rivet logo' />
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
            <Link href='/discord'>
              <Icon icon={faDiscord} />
            </Link>
          </RivetHeader.NavItem>
          <RivetHeader.NavItem asChild className='p-2'>
            <Link href='https://github.com/rivet-gg/rivet' target='_blank'>
              <Icon icon={faGithub} />
            </Link>
          </RivetHeader.NavItem>
          <Button variant='outline' asChild className='font-v2 text-foreground subpixel-antialiased'>
            <Link href='https://hub.rivet.gg'>Sign In</Link>
          </Button>
        </>
      }
      mobileBreadcrumbs={<DocsMobileNavigation />}
      breadcrumbs={
        <div
          className='flex items-center gap-6 font-v2 subpixel-antialiased'
          ref={node => setRef(node?.closest('header')?.querySelector('div:first-child') || null)}>
          <TooltipProvider delayDuration={0} skipDelayDuration={0}>
            <Tooltip key='product'>
              <TooltipTrigger asChild>
                <div>
                  <RivetHeader.NavItem className='flex items-center gap-1 py-2'>
                    <Link href='/docs' aria-current={active === 'product' ? 'page' : undefined}>
                      Product
                    </Link>
                  </RivetHeader.NavItem>
                </div>
              </TooltipTrigger>
              <TooltipPortal>
                <TooltipContent
                  collisionPadding={32}
                  collisionBoundary={ref}
                  style={{ width: 'calc(var(--radix-popper-available-width)' }}
                  className='h-full max-h-[190px] max-w-[800px] bg-card p-4'>
                  <TooltipArrow className='h-2.5 w-5 fill-border' />
                  <div className='h-full bg-card'>
                    <HeaderPopupProductMenu />
                  </div>
                </TooltipContent>
              </TooltipPortal>
            </Tooltip>
            {/* <Tooltip delayDuration={0} key='solutions'>
              <TooltipTrigger asChild>
                <div>
                  <RivetHeader.NavItem asChild className='flex items-center gap-1 py-2'>
                    <Link href='/docs' aria-current={active === 'solutions' ? 'page' : undefined}>
                      Solutions
                    </Link>
                  </RivetHeader.NavItem>
                </div>
              </TooltipTrigger>
              <TooltipPortal>
                <TooltipContent
                  collisionPadding={32}
                  collisionBoundary={ref}
                  style={{ width: 'calc(var(--radix-popper-available-width)' }}
                  className='flex h-full min-h-[190px] max-w-[800px] bg-card p-4'>
                  <TooltipArrow className='h-2.5 w-5 fill-border' />
                  <div className='flex-1 justify-items-stretch bg-card'>
                    <HeaderPopupSolutionsMenu />
                  </div>
                </TooltipContent>
              </TooltipPortal>
            </Tooltip> */}
          </TooltipProvider>

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
