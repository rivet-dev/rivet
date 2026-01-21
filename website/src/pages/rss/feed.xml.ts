import type { APIRoute } from 'astro';
import { getCollection } from 'astro:content';
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

	// Filter out unpublished posts
	const publishedPosts = posts.filter(p => !p.data.unpublished);

	for (const post of publishedPosts) {
		const slug = post.id.replace(/\/page$/, '');
		const url = `${siteUrl}/blog/${slug}/`;
		const author = AUTHORS[post.data.author];

		const title = post.data.title;

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
