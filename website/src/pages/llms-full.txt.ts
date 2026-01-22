import type { APIRoute } from 'astro';
import { getCollection } from 'astro:content';

export const prerender = true;

/**
 * Strip MDX/JSX syntax to get clean markdown content
 */
function cleanMdxContent(content: string): string {
	return content
		// Remove import statements
		.replace(/import\s+.*?from\s+['"][^'"]*['"];?\s*/g, '')
		// Remove export statements
		.replace(/export\s+(?:default\s+)?(?:const|let|var|function|class)?\s*[^;{]*[;{]?/g, '')
		// Remove JSX components (self-closing and with children)
		.replace(/<[A-Z][a-zA-Z0-9]*[^>]*\/>/g, '')
		.replace(/<[A-Z][a-zA-Z0-9]*[^>]*>[\s\S]*?<\/[A-Z][a-zA-Z0-9]*>/g, '')
		// Remove HTML comments
		.replace(/<!--[\s\S]*?-->/g, '')
		// Remove JSX expressions
		.replace(/\{[^}]*\}/g, '')
		// Remove remaining HTML/JSX tags
		.replace(/<[^>]+>/g, '')
		// Normalize multiple line breaks
		.replace(/\n\s*\n\s*\n/g, '\n\n')
		.trim();
}

/**
 * Extract content starting from the first h1 heading
 */
function stripContentBeforeFirstH1(content: string): string {
	const h1Match = content.match(/^#\s+(.+)$/m);
	if (!h1Match || h1Match.index === undefined) {
		return content;
	}
	return content.substring(h1Match.index);
}

/**
 * Get the title from content or frontmatter
 */
async function getTitle(entry: { id: string; data: { title?: string }; body?: string }, renderedHeadings: { depth: number; text: string }[]): Promise<string> {
	// Check frontmatter first
	if (entry.data.title) {
		return entry.data.title;
	}

	// Check rendered headings
	const h1 = renderedHeadings.find(h => h.depth === 1);
	if (h1) {
		return h1.text;
	}

	// Fall back to ID
	return entry.id.split('/').pop() || entry.id;
}

export const GET: APIRoute = async () => {
	const docs = await getCollection('docs');

	// Filter out cloud docs and sort by path
	const filteredDocs = docs
		.filter(doc => !doc.id.startsWith('cloud'))
		.sort((a, b) => a.id.localeCompare(b.id));

	// Process each doc
	const processedDocs = await Promise.all(
		filteredDocs.map(async (doc) => {
			const title = doc.data.title as string;

			// Get raw body content and clean it
			let cleanContent = '';
			if (doc.body) {
				const contentFromH1 = stripContentBeforeFirstH1(doc.body);
				cleanContent = cleanMdxContent(contentFromH1);
			}

			return {
				title,
				content: cleanContent,
				path: doc.id,
			};
		})
	);

	// Build the full content
	const content = [
		'# Rivet Documentation - Complete',
		'',
		'This file contains the complete documentation for Rivet, an open-source alternative to Durable Objects.',
		'',
		...processedDocs.map(doc => `## ${doc.title}\n\n${doc.content}`),
	].join('\n');

	return new Response(content, {
		headers: {
			'Content-Type': 'text/plain; charset=utf-8',
		},
	});
};
