import type { APIRoute } from 'astro';
import { getCollection, render } from 'astro:content';
import { Feed } from 'feed';
import { AUTHORS } from '@/lib/article';

// Ensure this route is pre-rendered at build time
export const prerender = true;

export const GET: APIRoute = async ({ site }) => {
	const siteUrl = site?.toString() || 'https://rivet.gg';
	const posts = await getCollection('posts');

	const feed = new Feed({
		title: 'Rivet',
		description: 'Rivet news',
		id: siteUrl,
		link: siteUrl,
		image: `${siteUrl}/favicon.ico`,
		favicon: `${siteUrl}/favicon.ico`,
		copyright: `All rights reserved ${new Date().getFullYear()} Rivet Gaming, Inc.`,
		feedLinks: {
			rss2: `${siteUrl}/rss/feed.xml`,
		},
	});

	for (const post of posts) {
		const slug = post.id.replace(/\/page$/, '');
		const url = `${siteUrl}/blog/${slug}/`;
		const author = AUTHORS[post.data.author];

		// Get title from headings
		const { headings } = await render(post);
		const title = headings.find(h => h.depth === 1)?.text || slug;

		feed.addItem({
			title,
			id: slug,
			date: post.data.published,
			author: [{ name: author.name }],
			link: url,
			description: '',
		});
	}

	return new Response(feed.rss2(), {
		headers: {
			'Content-Type': 'application/xml; charset=utf-8',
		},
	});
};
