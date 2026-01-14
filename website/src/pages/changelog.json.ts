import type { APIRoute } from 'astro';
import { getCollection, render } from 'astro:content';
import { AUTHORS, CATEGORIES } from '@/lib/article';

// Ensure this route is pre-rendered at build time
export const prerender = true;

export const GET: APIRoute = async () => {
	const posts = await getCollection('posts');
	const changelogPosts = posts.filter(p => p.data.category === 'changelog');

	// Import all post images eagerly
	const images = import.meta.glob<{ default: ImageMetadata }>(
		'/src/content/posts/*/image.{png,jpg,gif}',
		{ eager: true }
	);

	const entries = await Promise.all(
		changelogPosts
			.sort((a, b) => b.data.published.getTime() - a.data.published.getTime())
			.map(async (entry) => {
				const author = AUTHORS[entry.data.author];
				const { headings } = await render(entry);
				const title = headings.find(h => h.depth === 1)?.text || entry.id;
				const slug = entry.id.replace(/\/page$/, '');

				// Find the image for this post
				const imagePath = Object.keys(images).find(p => p.includes(slug));
				const image = imagePath ? images[imagePath].default : null;

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

interface ImageMetadata {
	src: string;
	width: number;
	height: number;
	format: string;
}
