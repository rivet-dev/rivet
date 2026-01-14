import type { OGImageOptions } from 'astro-og-canvas';

// Common OG image options for all pages
export function getOgImageOptions(
	title: string,
	description?: string
): OGImageOptions {
	return {
		title: title,
		description: description || undefined,
		logo: {
			path: './public/icons/android-chrome-512x512.png',
			size: [80],
		},
		bgGradient: [[15, 15, 15], [25, 25, 30]],
		border: {
			color: [50, 50, 55],
			width: 20,
			side: 'inline-start',
		},
		padding: 60,
		font: {
			title: {
				color: [240, 240, 240],
				size: 72,
				lineHeight: 1.2,
			},
			description: {
				color: [160, 160, 165],
				size: 36,
				lineHeight: 1.4,
			},
		},
	};
}

// Get section-specific styling
export function getSectionLabel(section: string): string {
	const labels: Record<string, string> = {
		docs: 'Documentation',
		blog: 'Blog',
		changelog: 'Changelog',
		guides: 'Guides',
		learn: 'Learn',
	};
	return labels[section] || section;
}
