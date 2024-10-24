import { loadArticles } from '@/lib/article';
import { getSiteUrl } from '@/lib/siteUrl';

import { Feed } from 'feed';
import { NextResponse } from 'next/server';

export async function GET() {
  let siteUrl = getSiteUrl();

  let articles = await loadArticles();

  let feed = new Feed({
    title: 'Rivet',
    description: 'Rivet news',
    id: siteUrl,
    link: siteUrl,
    image: `${siteUrl}/favicon.ico`,
    favicon: `${siteUrl}/favicon.ico`,
    copyright: `All rights reserved ${new Date().getFullYear()} Rivet Gaming, Inc.`,
    feedLinks: {
      rss2: `${siteUrl}/rss/feed.xml`
    }
  });

  articles.forEach(article => {
    let url = `${siteUrl}/blog/${article.slug}`;
    feed.addItem({
      title: article.title,
      id: article.slug,
      date: article.published,
      author: article.author.name,
      link: url,
      description: article.description
    });
  });

  let response = new NextResponse(feed.rss2());
  response.headers.set('Content-Type', 'application/xml');
  return response;
}
