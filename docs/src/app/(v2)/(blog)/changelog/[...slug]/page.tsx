import { Prose } from '@/components/Prose';
import { DocsTableOfContents } from '@/components/DocsTableOfContents';
import { Avatar, AvatarFallback, AvatarImage } from '@rivet-gg/components';
import { Icon, faCalendarDay, faChevronRight } from '@rivet-gg/icons';
import Link from 'next/link';
import Image from 'next/image';
import { generateArticlesPageParams, loadArticle, loadArticles } from '@/lib/article';
import { formatTimestamp } from '@/lib/formatDate';
import { ArticleSocials } from '@/components/ArticleSocials';
import { Metadata } from 'next';

export async function generateMetadata({ params: { slug } }): Promise<Metadata> {
  const { description, title, author, published, tags, category, image } = await loadArticle(slug.join('/'));

  return {
    title,
    description,
    authors: [author],
    keywords: tags,
    openGraph: {
      title,
      description,
      type: 'article',
      publishedTime: new Date(published).toISOString(),
      authors: [author.name],
      section: category.name,
      tags,
      images: [
        {
          url: image.src,
          width: image.width,
          height: image.height,
          alt: image.alt
        }
      ]
    }
  };
}

export default async function BlogPage({ params: { slug } }) {
  const { Content, title, tableOfContents, author, published, category, image } = await loadArticle(
    slug.join('/')
  );

  return (
    <>
      <ul className='text-muted-foreground my-4 flex flex-wrap items-center gap-2 text-xs'>
        <li>
          <Link href='/changelog'>Changelog</Link>
        </li>
        <li className='h-2.5'>
          <Icon className='block h-full w-auto' icon={faChevronRight} />
        </li>
        <li className='text-foreground font-semibold'>{title}</li>
      </ul>
      <div className='flex flex-col gap-4 pb-6 lg:flex-row'>
        <aside className='order-1 mt-2 flex min-w-0 max-w-xs flex-1 flex-col gap-2'>
          <div className='top-header sticky pt-2'>
            <p className='mb-2 text-sm font-semibold'>Posted by</p>
            <div className='mb-4 flex items-center gap-2'>
              <Avatar>
                <AvatarFallback>{author[0]}</AvatarFallback>
                <AvatarImage {...author.avatar} alt={author} />
              </Avatar>
              <div className='flex flex-col'>
                <h3 className='font-bold'>{author.name}</h3>
                <p className='text-muted-foreground text-sm'>{author.role}</p>
              </div>
            </div>
            <div className='text-muted-foreground mb-6 flex items-center gap-2 text-sm'>
              <Icon icon={faCalendarDay} />
              <time className='text-sm'>{formatTimestamp(published)}</time>
            </div>
            <p className='mb-2 hidden text-sm font-semibold lg:block'>Other entries</p>
            <OtherArticles slug={slug[0]} />
          </div>
        </aside>
        <Prose as='article' className='order-3 mt-4 w-full max-w-prose flex-shrink-0 lg:order-2'>
          <Image {...image} alt='Promo Image' className='rounded-sm border' />
          <Content />
          <ArticleSocials title={title} />
        </Prose>
        <aside className='order-2 min-w-0 max-w-xs flex-1 lg:order-3'>
          <DocsTableOfContents tableOfContents={tableOfContents} />
        </aside>
      </div>
    </>
  );
}

export function generateStaticParams() {
  return generateArticlesPageParams();
}

async function OtherArticles({ slug }) {
  const articles = await loadArticles();

  const entries = articles.filter(article => article.category.id === 'changelog');

  return (
    <ul className='text-muted-foreground hidden list-disc pl-5 text-sm lg:block'>
      {entries
        .filter(article => article.slug !== slug)
        .sort((a, b) => b.published - a.published)
        .slice(0, 3)
        .map(article => (
          <li key={article.slug} className='py-1'>
            <Link href={`/blog/${article.slug}`}>{article.title}</Link>
          </li>
        ))}
    </ul>
  );
}
