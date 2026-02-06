import { getCollection, render } from 'astro:content';
import { OGImageRoute } from 'astro-og-canvas';
import { getOgImageOptions } from '@/lib/og-config';

const learn = await getCollection('learn');

// Build pages object with titles and descriptions
const pages = Object.fromEntries(
	await Promise.all(
		learn.map(async (entry) => {
			const { headings } = await render(entry);
			const title = entry.data.title || headings.find((h) => h.depth === 1)?.text || 'Learn';
			const description = entry.data.subtitle || entry.data.description || 'Learn Rivet';
			// Handle index files
			const slug = entry.id === 'index' ? '' : entry.id.replace(/\/index$/, '');
			return [slug, { title, description, act: entry.data.act }];
		})
	)
);

export const { getStaticPaths, GET } = await OGImageRoute({
	param: 'slug',
	pages: pages,
	getImageOptions: (_path, page) => {
		const description = page.act ? `${page.act} - ${page.description}` : page.description;
		return getOgImageOptions(page.title, description);
	},
});
