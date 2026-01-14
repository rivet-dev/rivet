import { getCollection, render } from 'astro:content';
import { OGImageRoute } from 'astro-og-canvas';
import { getOgImageOptions } from '@/lib/og-config';

const guides = await getCollection('guides');

// Build pages object with titles and descriptions
const pages = Object.fromEntries(
	await Promise.all(
		guides.map(async (entry) => {
			const { headings } = await render(entry);
			const title = entry.data.title || headings.find((h) => h.depth === 1)?.text || 'Guide';
			const description = entry.data.description || 'Rivet Guide';
			return [entry.id, { title, description }];
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
