import { getCollection, render } from 'astro:content';
import { OGImageRoute } from 'astro-og-canvas';
import { getOgImageOptions } from '@/lib/og-config';
import { CATEGORIES } from '@/lib/article';

const posts = await getCollection('posts');

// Build pages object with titles and descriptions for blog posts (non-changelog)
const pages = Object.fromEntries(
	await Promise.all(
		posts
			.filter((post) => post.data.category !== 'changelog')
			.map(async (entry) => {
				const { headings } = await render(entry);
				const slug = entry.id.replace(/\/page$/, '');
				const title = headings.find((h) => h.depth === 1)?.text || slug;
				const category = CATEGORIES[entry.data.category];
				const description = category?.name || 'Blog';
				return [slug, { title, description }];
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
