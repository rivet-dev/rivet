import { ArticleInfo } from '@/lib/articles/metadata';
import Link from 'next/link';

interface OtherArticlesListProps {
  currentSlug: string;
}

export const OtherArticlesList = async ({ currentSlug }: OtherArticlesListProps) => {
  const routes = await import('@/generated/routes.json');

  const articles = Object.fromEntries(
    Object.entries(routes.pages).filter(([path]) => path.startsWith('/blog') && !path.endsWith(currentSlug))
  );

  const richArticlesEntries: [string, ArticleInfo][] = await Promise.all(
    Object.entries(articles).map(async ([path, page]) => {
      const post = await import(`../app/(legacy)/blog/(posts)/${path.replace('/blog/', '')}/page.mdx`);
      return [path, { ...page, ...post.info } as ArticleInfo];
    })
  );

  richArticlesEntries.sort(([, a], [, b]) => b.date.getTime() - a.date.getTime());

  const richArticles: Record<string, ArticleInfo> = Object.fromEntries(richArticlesEntries);

  const formatter = new Intl.DateTimeFormat('en', {});

  return (
    <ul className='mt-2 hidden text-sm text-cream-100 xl:block'>
      {Object.entries(richArticles).map(([path, article]) => {
        return (
          <li key={path} className='mb-3 flex'>
            <Link href={path} className='hover:text-cream-300'>
              <p className='text-xs leading-tight'>{article.title}</p>
              <div className='text-2xs text-charcole-800'>
                {article.author.name} @ <i>{formatter.format(new Date(article.date))}</i>
              </div>
            </Link>
          </li>
        );
      })}
    </ul>
  );
};
