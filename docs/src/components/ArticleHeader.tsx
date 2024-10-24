import { ArticleInfo } from '@/lib/articles/metadata';
import { formatTimestamp } from '@/lib/formatDate';
import Image from 'next/image';

export function ArticleHeader({ title, date, author, images }: ArticleInfo) {
  return (
    <header>
      <div className='relative w-full'>
        <Image src={images.hero.image} alt={images.hero.alt} className='aspect-[2/1] w-full object-cover' />
        <div className='pointer-events-none absolute inset-0 border-4 border-white/80'></div>
      </div>

      {/* Title */}
      <h1 className='tracking-tigh mt-6 text-4xl font-bold text-cream-100 sm:text-5xl'>{title}</h1>

      {/* Date */}
      <time
        dateTime={formatTimestamp(date)}
        className='order-first flex items-center text-base  text-charcole-500'>
        <span className='h-4 w-0.5 rounded-full bg-cream-500' />
        <span className='ml-3'>{formatTimestamp(date)}</span>
      </time>
    </header>
  );
}
