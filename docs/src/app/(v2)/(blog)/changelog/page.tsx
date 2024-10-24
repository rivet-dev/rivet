import { Icon, faRss, faSparkle } from '@rivet-gg/icons';
import { formatTimestamp } from '@/lib/formatDate';
import Link from 'next/link';
import Image from 'next/image';
import { Metadata } from 'next';
import { loadArticles } from '@/lib/article';
import {
  Button,
  Avatar,
  AvatarFallback,
  AvatarImage,
  TabsList,
  TabsTrigger,
  Tabs
} from '@rivet-gg/components';
import { Prose } from '@/components/Prose';

export const metadata: Metadata = {
  title: 'Changelog - Rivet'
};

function Article({ slug, published, author, image, Content }) {
  let href = `/changelog/${slug}`;
  return (
    <div className='group/article relative flex size-full flex-col items-start justify-between pb-14 pl-10'>
      <div className='after:border-border absolute inset-y-0 -left-5 top-1 after:absolute after:inset-y-0 after:left-1/2 after:-z-[1] after:w-[1px] after:-translate-x-1/2 after:border-l after:content-[""] group-last/article:after:hidden'>
        <div className='bg-foreground text-background flex size-5 items-center justify-center rounded-full p-4'>
          <Icon icon={faSparkle} />
        </div>
      </div>
      <div className='flex w-full flex-col'>
        {/* Date & category */}
        <div className='mb-4 flex items-center gap-x-3 text-xs'>
          <div className='relative flex items-center gap-x-4'>
            <Avatar>
              <AvatarFallback>{author[0]}</AvatarFallback>
              <AvatarImage {...author.avatar} alt={author.name} />
            </Avatar>
            <div className='text-sm'>
              <div className='font-semibold'>{author.name}</div>
              <time dateTime={formatTimestamp(published)} className='text-muted-foreground'>
                {formatTimestamp(published)}
              </time>
            </div>
          </div>
        </div>

        <Link href={href} className='size-full'>
          {/* Image */}
          <div className='relative w-full'>
            <Image src={image} alt={'hero'} className='aspect-[2/1] w-full rounded-md border object-cover' />
          </div>
        </Link>

        {/* Description */}
        <div className='relative mt-3'>
          <Prose>
            <Content
              components={{
                h1: props => (
                  <h2 {...props} className='group-hover/article:underline'>
                    <Link href={href} className='no-underline'>
                      {props.children}
                    </Link>
                  </h2>
                ),
                h2: props => <h3 {...props} />,
                h3: props => <h4 {...props} />
              }}
            />
          </Prose>
        </div>
      </div>
    </div>
  );
}

export default async function BlogPage() {
  const articles = await loadArticles();

  const entries = articles.filter(article => article.category.id === 'changelog');
  return (
    <>
      <div className='mt-8 flex w-full items-center justify-center'>
        <h1 className='text-6xl font-bold'>Changelog</h1>
      </div>
      <div className='mb-8 mt-4 flex items-center justify-center'>
        <div className='bg-card rounded-md border'>
          <Button variant='ghost' asChild>
            <Link href='/blog'>All Posts</Link>
          </Button>
          <Button variant='secondary' asChild>
            <Link href='/changelog'>Changelog</Link>
          </Button>
        </div>
      </div>
      <div className='mx-auto mb-8 mt-16 flex max-w-prose flex-col'>
        {entries
          .sort((a, b) => b.published - a.published)
          .map(article => (
            <Article key={article.slug} {...article} />
          ))}
      </div>
    </>
  );
}
