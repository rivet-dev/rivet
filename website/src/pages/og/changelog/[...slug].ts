import { getCollection, render } from 'astro:content';
import { OGImageRoute } from 'astro-og-canvas';
import { getOgImageOptions } from '@/lib/og-config';

const posts = await getCollection('posts');

// Build pages object with titles for changelog posts
const pages = Object.fromEntries(
	await Promise.all(
		posts
			.filter((post) => post.data.category === 'changelog')
			.map(async (entry) => {
				const { headings } = await render(entry);
				const slug = entry.id.replace(/\/page$/, '');
				const title = headings.find((h) => h.depth === 1)?.text || slug;
				return [slug, { title, description: 'Changelog' }];
			})
	)
);

export const { getStaticPaths, GET } = await OGImageRoute({
	param: 'slug',
	pages: pages,
	getImageOptions: (_path, page) => {
		return getOgImageOptions(page.title, page.description);
	},
});
