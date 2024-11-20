import { Header } from '@/components/v2/Header';
import { ModulePageLink } from '@/components/ModulePageLink';
import { CSSProperties } from 'react';
import { sitemap } from '@/sitemap/mod';
import { pagesContainsHref } from '@/lib/sitemap';

function Subnav({ slug }) {
  const fullPath = `/docs/${slug.join('/')}`;
  return (
    <div className='-mx-8 -mb-[9px] hidden min-h-10 items-center px-8 empty:hidden md:flex'>
	  {sitemap.map(({ title, href, sidebar }, i) => (
		  <ModulePageLink key={i} href={href} isActive={pagesContainsHref(fullPath, sidebar)}>{title}</ModulePageLink>
	  ))}
    </div>
  );
}

export default function Layout({ params: { slug }, children }) {
  return (
    <>
      <Header active='docs' subnav={<Subnav slug={slug} />} />
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
