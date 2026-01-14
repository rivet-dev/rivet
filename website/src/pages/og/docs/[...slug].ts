import { getCollection, render } from 'astro:content';
import { OGImageRoute } from 'astro-og-canvas';
import { getOgImageOptions } from '@/lib/og-config';

const docs = await getCollection('docs');

// Build pages object with titles and descriptions
const pages = Object.fromEntries(
	await Promise.all(
		docs.map(async (entry) => {
			const { headings } = await render(entry);
			const title = entry.data.title || headings.find((h) => h.depth === 1)?.text || 'Documentation';
			const description = entry.data.description || '';
			// Handle index files - use empty string for root
			const slug = entry.id === 'index' ? '' : entry.id.replace(/\/index$/, '');
			return [slug, { title, description }];
		})
	)
);

export const { getStaticPaths, GET } = await OGImageRoute({
	param: 'slug',
	pages: pages,
	getImageOptions: (_path, page) => {
		return getOgImageOptions(page.title, page.description || 'Rivet Documentation');
	},
});
