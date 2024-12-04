/**
 * This file is a proxy for the MDX files in the docs directory.
 * It loads the MDX file based on the slug and renders it.
 * It also generates the metadata for the page.
 * We avoid using the new `page.mdx` convention because its harder to navigate the docs when editing.
 * Also, importing the MDX files directly allow us to use other exports from the MDX files.
 */

import path from 'node:path';
import fs from 'node:fs/promises';
import { Prose } from '@/components/Prose';
import { Metadata } from 'next';
import { DocsTableOfContents } from '@/components/DocsTableOfContents';
import { DocsNavigation } from '@/components/DocsNavigation';
import { sitemap } from '@/sitemap/mod';
import { findActiveTab } from '@/lib/sitemap';
import { buildFullPath, buildPathComponents as buildPathComponents, VALID_SECTIONS } from './util';
import { notFound } from 'next/navigation';

interface Param {
  section: string;
  page?: string[];
}

function createParamsForFile(section, file): Param {
  return {
    section,
    page: [...file.replace('index.mdx', '').replace('.mdx', '').split('/').filter(x => x.length > 0)]
  };
}

async function loadContent(path: string[]) {
  try {
    return await import('@/content/' + path.join('/') + '.mdx');
  } catch (error) {
    if (error.code === 'MODULE_NOT_FOUND') {
      try {
        return await import('@/content/' + path.join('/') + '/index.mdx');
      } catch (indexError) {
        if (indexError.code === 'MODULE_NOT_FOUND') {
          throw new Error(`Content not found for path: ${path.join('/')}`);
        } else {
          throw indexError;
        }
      }
    } else {
      throw error;
    }
  }
}

export async function generateMetadata({ params: { section, page } }): Promise<Metadata> {
  const path = buildPathComponents(section, page);
  const { title, description } = await loadContent(path);

  return {
    title: `${title} - Rivet`,
    description,
  };
}

export default async function CatchAllCorePage({ params: { section, page } }) {
  if (!VALID_SECTIONS.includes(section)) {
    notFound();
  }

  const path = buildPathComponents(section, page);
  const { default: Content, tableOfContents } = await loadContent(path);

  const fullPath = buildFullPath(path);
  const foundTab = findActiveTab(fullPath, sitemap);
  const parentPage = foundTab?.page.parent;

  return (
    <>
      <aside className='hidden md:block'>
        {foundTab?.tab.sidebar ? <DocsNavigation sidebar={foundTab.tab.sidebar} /> : null}
      </aside>
      <main className='mx-auto mt-8 w-full max-w-prose px-8 pb-8'>
        <Prose as='article'>
          {parentPage && <div className='eyebrow h-5 text-primary text-sm font-semibold'>{parentPage.title}</div>}
          <Content />
        </Prose>
      </main>
      <aside className='-order-1 mx-auto w-full min-w-0 max-w-3xl flex-shrink-0 pb-4 pl-4 md:order-none xl:mx-0'>
        <DocsTableOfContents className='lg:max-h-content' tableOfContents={tableOfContents} />
      </aside>
    </>
  );
}

export async function generateStaticParams() {
  let staticParams: Param[] = [];
  for (const section of VALID_SECTIONS) {
    const dir = path.join(process.cwd(), 'src', 'content', section);

    const dirs = await fs.readdir(dir, { recursive: true });
    const files = dirs.filter(file => file.endsWith('.mdx'));

    staticParams.push(...files.map(file => {
      return createParamsForFile(section, file);
    }));
  }

  return staticParams;
}
