import type { APIRoute } from 'astro';
import { getCollection } from 'astro:content';
import { AUTHORS, CATEGORIES } from '@/lib/article';
import { getPostImage } from '@/lib/postImage';

// Ensure this route is pre-rendered at build time
export const prerender = true;

export const GET: APIRoute = async () => {
	const posts = await getCollection('posts');
	const changelogPosts = posts.filter(p => p.data.category === 'changelog' && !p.data.unpublished);

	const entries = await Promise.all(
		changelogPosts
			.sort((a, b) => b.data.published.getTime() - a.data.published.getTime())
			.map(async (entry) => {
				const author = AUTHORS[entry.data.author];
				const title = entry.data.title;
				const slug = entry.id.replace(/\/page$/, '');

				const image = getPostImage(entry);

				return {
					title,
					description: '',
					slug,
					published: entry.data.published,
					authors: [{
						name: author.name,
						role: author.role,
						avatar: {
							url: author.avatar?.src || '',
							height: author.avatar?.height || 0,
							width: author.avatar?.width || 0,
						},
					}],
					section: CATEGORIES[entry.data.category].name,
					tags: entry.data.keywords || [],
					images: image ? [{
						url: image.src,
						width: image.width,
						height: image.height,
						alt: title,
					}] : [],
				};
			})
	);

	return new Response(JSON.stringify(entries), {
		headers: {
			'Content-Type': 'application/json; charset=utf-8',
		},
	});
};
