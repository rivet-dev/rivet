'use client';

import { ActiveLink } from '@/components/ActiveLink';
import { Tree } from '@/components/DocsNavigation';
import { sitemap } from '@/sitemap/mod';
import { Header as RivetHeader } from '@rivet-gg/components/header';
import { usePathname } from 'next/navigation';

function CoreNavigation() {}

export function DocsMobileNavigation() {
  const pathname = usePathname() || '';

  const currentPage = sitemap.find(page => pathname.startsWith(page.href));
  return (
    <>
      <RivetHeader.NavItem asChild className='flex items-center gap-1.5'>
        <ActiveLink href='/docs'>Docs</ActiveLink>
      </RivetHeader.NavItem>
	  {sitemap.map(({ title, href, sidebar }) => {
		  const isActive = pagesContainsHref(currentPage?.href, sidebar);
			return (
			  <div key={title} className='ml-2'>
					<RivetHeader.NavItem asChild>
					  <ActiveLink href='/docs/html5' isActive={isActive}>HTML5</ActiveLink>
					</RivetHeader.NavItem>
					{isActive && <Tree pages={sidebar} className='mt-2' />}
			  </div>
			);
      })}
      <RivetHeader.NavItem asChild className='flex items-center gap-1.5'>
        <ActiveLink href='/modules'>Modules</ActiveLink>
      </RivetHeader.NavItem>
      <RivetHeader.NavItem asChild className='flex items-center gap-1.5'>
        <ActiveLink href='/changelog'>Changelog</ActiveLink>
      </RivetHeader.NavItem>
      <RivetHeader.NavItem asChild className='flex items-center gap-1.5'>
        <ActiveLink href='/pricing'>Pricing</ActiveLink>
      </RivetHeader.NavItem>
    </>
  );
}
