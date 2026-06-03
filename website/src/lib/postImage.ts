import type { CollectionEntry } from 'astro:content';

// Hero images live in R2 under a fixed convention keyed by the post slug, so the
// URL is derived from the slug instead of being written out in frontmatter. See
// the asset conventions in the root CLAUDE.md.
const ASSETS_BASE = 'https://assets.rivet.dev';

// Hero images are authored at a fixed 2:1 ratio. We assume these dimensions
// rather than reading them from each remote file so the build stays hermetic
// (no network calls to probe image sizes). The values feed the `<img>` intrinsic
// hint and the changelog feed; they are rendered responsively, so the displayed
// size is controlled by CSS.
export const HERO_IMAGE_WIDTH = 2048;
export const HERO_IMAGE_HEIGHT = 1024;

export interface PostImage {
	src: string;
	width: number;
	height: number;
}

// Resolve a post's hero image from its `image` frontmatter flag. The presence of
// the flag means the post has a hero image; the file is always named
// `image.{format}` under the post's slug. Returns null when the post has no hero.
export function getPostImage(entry: CollectionEntry<'posts'>): PostImage | null {
	const config = entry.data.image;
	if (!config) return null;

	const slug = entry.id.replace(/\/page$/, '');
	const format = typeof config === 'object' && config.format ? config.format : 'png';
	const file = typeof config === 'object' && config.file ? config.file : `image.${format}`;

	return {
		src: `${ASSETS_BASE}/website/blog/${slug}/${file}`,
		width: HERO_IMAGE_WIDTH,
		height: HERO_IMAGE_HEIGHT,
	};
}
