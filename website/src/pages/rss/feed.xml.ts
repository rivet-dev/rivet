import type { APIRoute } from 'astro';
import { getCollection } from 'astro:content';
import { Feed } from 'feed';
import { AUTHORS } from '@/lib/article';

// Ensure this route is pre-rendered at build time
export const prerender = true;

export const GET: APIRoute = async ({ site }) => {
	// site.toString() normalizes to a trailing slash; strip it so concatenated
	// paths below don't produce double slashes (e.g. https://rivet.dev//blog/).
	const siteUrl = (site?.toString() || 'https://rivet.dev').replace(/\/$/, '');
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

	// Filter out unpublished posts and sort newest first. The feed lib emits
	// items in insertion order, so we sort before adding.
	const publishedPosts = posts
		.filter(p => !p.data.unpublished)
		.sort((a, b) => b.data.published.getTime() - a.data.published.getTime());

	for (const post of publishedPosts) {
		const slug = post.id.replace(/\/page$/, '');
		// Changelog posts live under /changelog/, all other posts under /blog/.
		const prefix = post.data.category === 'changelog' ? 'changelog' : 'blog';
		const url = `${siteUrl}/${prefix}/${slug}/`;
		const author = AUTHORS[post.data.author];

		const title = post.data.title;

		feed.addItem({
			title,
			id: slug,
			date: post.data.published,
			author: [{ name: author.name }],
			link: url,
			description: post.data.description,
		});
	}

	return new Response(feed.rss2(), {
		headers: {
			'Content-Type': 'application/xml; charset=utf-8',
		},
	});
};
