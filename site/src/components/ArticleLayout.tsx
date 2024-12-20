import { Prose } from '@/components/Prose';
import { TableOfContents } from '@/components/TableOfContents';
import { ArticleInfo } from '@/lib/articles/metadata';
import { ReactNode } from 'react';
import Image from 'next/image';
import { ArticleHeader } from '@/components/ArticleHeader';
import { OtherArticlesList } from '@/components/OtherArticlesList';

interface ArticleLayoutProps {
  children: ReactNode;
  info: ArticleInfo;
}

export const ArticleLayout = ({ children, info }: ArticleLayoutProps) => {
  return (
    <>
      <aside className='mx-auto w-full max-w-3xl flex-1 xl:max-w-aside xl:pb-8'>
        <div className='mt-8'>
          <p className='mb-3 mt-0  font-sans text-xs font-semibold text-white'>Posted by</p>
          {/* Author */}
          <div className='relative flex items-center gap-x-3'>
            <Image
              src={info.author.avatar}
              width={50}
              height={50}
              alt={info.author.name}
              className='h-12 w-12 rounded-full bg-cream-100'
            />
            <div>
              <div className='text-md font-semibold text-cream-100'>{info.author.name}</div>
              <div className='text-sm text-cream-300'>{info.author.role}</div>
            </div>
          </div>
        </div>

        <p className='mt-6 hidden font-sans text-xs font-semibold text-white xl:block'>Other articles</p>
        <OtherArticlesList currentSlug={info.slug} />
      </aside>
      <Prose as='article' className='order-2 mx-auto max-w-3xl flex-1 pb-8'>
        <ArticleHeader {...info} />
        {children}
      </Prose>
      <aside className='col-start-2 row-span-3 mx-auto w-full max-w-3xl pb-2 xl:order-3 xl:max-w-aside'>
        <TableOfContents />
      </aside>
    </>
  );
};
