/**
 * This file is a proxy for the MDX files in the docs directory.
 * It loads the MDX file based on the slug and renders it.
 * It also generates the metadata for the page.
 * We avoid using the new `page.mdx` convention because its harder to navigate the docs when editing.
 * Also, importing the MDX files directly allow us to use other exports from the MDX files.
 */

import path from 'node:path';
import fs from 'node:fs/promises';
import { CORE_DIRECTORIES, ENGINES, ENGINE_LABEL_MAP, getAliasedSlug } from '@/lib/sameAs';
import { Prose } from '@/components/Prose';
import { Metadata } from 'next';
import { DocsTableOfContents } from '@/components/DocsTableOfContents';
import { DocsNavigation } from '@/components/DocsNavigation';
import { sitemap } from '@/sitemap/mod';
import { findActiveTab } from '@/lib/sitemap';

function createParamsForFile(file) {
  return {
    slug: [...file.replace('index.mdx', '').replace('.mdx', '').split('/')]
  };
}

async function loadContent(slug: string[]) {
  const newSlug = getAliasedSlug(slug);
  try {
    return await import('@/docs/' + newSlug.join('/') + '.mdx');
  } catch {
    return await import('@/docs/' + newSlug.join('/') + '/index.mdx');
  }
}

export async function generateMetadata({ params: { slug } }): Promise<Metadata> {
  const { title, description } = await loadContent(slug);

  return {
    title: `${title} - ${ENGINE_LABEL_MAP[slug[0]] || ENGINE_LABEL_MAP.default} - Rivet Docs`,
    description,
  };
}

export default async function CatchAllCorePage({ params: { slug } }) {
  const { default: Content, tableOfContents } = await loadContent(slug);

  const fullPath = `/docs/${slug.join('/')}`;
  const tab = findActiveTab(fullPath, sitemap)

  return (
    <>
      <aside className='hidden md:block'>
        {tab?.sidebar ? <DocsNavigation sidebar={tab.sidebar} /> : null}
      </aside>
      <main className='mx-auto mt-8 w-full max-w-prose px-8 pb-8'>
        <Prose as='article'>
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
  const dir = path.join(process.cwd(), 'src/docs');

  const dirs = await fs.readdir(dir, { recursive: true });
  const files = dirs.filter(file => file.endsWith('.mdx'));

  const staticParams = files.map(file => {
    return createParamsForFile(file);
  });

  const coreResources = CORE_DIRECTORIES.flatMap(dir => files.filter(file => file.startsWith(dir)));
  for (const engine of ENGINES) {
    staticParams.push(
      ...coreResources.map(file => {
        return createParamsForFile(`${engine}/${file}`);
      })
    );
  }

  staticParams.push(
    ...coreResources
      .filter(file => file.startsWith('modules'))
      .map(file => createParamsForFile(`general/${file}`))
  );

  staticParams.push(
    ...coreResources
      .filter(file => file.startsWith('general'))
      .map(file => createParamsForFile(`modules/${file}`))
  );

  return staticParams;
}
