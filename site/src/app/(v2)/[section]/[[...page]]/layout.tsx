import { Header } from '@/components/v2/Header';
import { ModulePageLink } from '@/components/ModulePageLink';
import { CSSProperties } from 'react';
import { sitemap } from '@/sitemap/mod';
import { findPageForHref } from '@/lib/sitemap';
import { buildFullPath, buildPathComponents } from './util';

function Subnav({ path }: { path: string[] }) {
  const fullPath = buildFullPath(path);
  return (
    <div className='-mx-8 -mb-[9px] hidden min-h-10 items-center px-8 empty:hidden md:flex'>
	  {sitemap.map((tab, i) => (
		  <ModulePageLink key={i} href={tab.href} isActive={findPageForHref(fullPath, tab)}>{tab.title}</ModulePageLink>
	  ))}
    </div>
  );
}

export default function Layout({ params: { section, page }, children }) {
  const path = buildPathComponents(section, page);
  return (
    <>
      <Header active='docs' subnav={<Subnav path={path} />} />
      <div className='flex w-full'>
        <div
          className='md:grid-cols-docs mx-auto flex flex-col md:grid md:px-6'
          style={{ '--header-height': '6.5rem' } as CSSProperties}>
          {children}
        </div>
      </div>
    </>
  );
}
