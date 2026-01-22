import type { AstroIntegration } from 'astro';
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import fg from 'fast-glob';
import { remark } from 'remark';
import { toString } from 'mdast-util-to-string';
import path from 'node:path';

interface PageData {
	title: string;
	description: string | null;
}

interface RoutesData {
	pages: Record<string, PageData>;
}

/**
 * Parse an MDX file to extract title and description
 */
async function processPage(filePath: string): Promise<PageData> {
	const content = await readFile(filePath, 'utf-8');

	// First check for YAML frontmatter
	const frontmatterMatch = content.match(/^---\n([\s\S]*?)\n---/);
	let title = '';
	let description: string | null = null;

	if (frontmatterMatch) {
		const frontmatter = frontmatterMatch[1];
		const titleMatch = frontmatter.match(/^title:\s*["']?([^"'\n]+)["']?$/m);
		const descMatch = frontmatter.match(/^description:\s*["']?([^"'\n]+)["']?$/m);
		if (titleMatch) title = titleMatch[1];
		if (descMatch) description = descMatch[1];
	}

	// If no title from frontmatter, extract from first heading
	if (!title) {
		const ast = remark().parse(content);

		const firstHeadingIndex = ast.children.findIndex(
			(node) => node.type === 'heading'
		);
		const firstHeading = ast.children[firstHeadingIndex];

		if (firstHeading && firstHeading.type === 'heading') {
			title = toString(firstHeading);
		}

		// Extract description from first paragraph after heading
		if (!description && firstHeadingIndex !== -1) {
			for (let i = firstHeadingIndex + 1; i < ast.children.length; i++) {
				const node = ast.children[i];
				if (node.type === 'paragraph') {
					description = toString(node);
					break;
				} else if (node.type === 'heading') {
					break;
				}
			}
		}
	}

	return { title, description };
}

/**
 * Convert file path to route href
 */
function filePathToHref(filePath: string): string {
	return '/' + filePath
		.replace(/\/index\.mdx$/, '')
		.replace(/\.mdx$/, '')
		.replace(/^pages\//, '')
		.replace(/^app\//, '')
		.replace(/\/page$/, '')
		.replace(/\(guide\)\//, '')
		.replace(/\(technical\)\//, '')
		.replace(/\(posts\)\//, '')
		.replace(/\(legacy\)\//, '');
}

/**
 * Astro integration that generates routes.json at build time
 */
export function generateRoutes(): AstroIntegration {
	return {
		name: 'generate-routes',
		hooks: {
			'astro:config:setup': async ({ config, logger }) => {
				const rootDir = process.cwd();
				const pages: Record<string, PageData> = {};

				logger.info('Generating routes.json...');

				// Find all MDX files
				const mdxFiles = await fg(
					['src/content/**/*.mdx'],
					{ cwd: rootDir }
				);

				for (const file of mdxFiles) {
					const filePath = path.join(rootDir, file);
					const href = filePathToHref(file);

					try {
						pages[href] = await processPage(filePath);
					} catch (error) {
						logger.warn(`Failed to process ${file}: ${error}`);
						pages[href] = { title: '', description: null };
					}
				}

				// Ensure generated directory exists
				const generatedDir = path.join(rootDir, 'src/generated');
				if (!existsSync(generatedDir)) {
					await mkdir(generatedDir, { recursive: true });
				}

				// Write routes.json
				const outputPath = path.join(generatedDir, 'routes.json');
				await writeFile(
					outputPath,
					JSON.stringify({ pages }, null, 2),
					'utf-8'
				);

				logger.info(`Generated ${Object.keys(pages).length} route entries`);

				// Generate individual markdown files in public/docs/
				await generateMarkdownFiles(rootDir, logger);
			},
		},
	};
}

/**
 * Generate individual markdown files in public/docs/
 */
async function generateMarkdownFiles(rootDir: string, logger: { info: (msg: string) => void; warn: (msg: string) => void }) {
	const docsDir = path.join(rootDir, 'src/content/docs');
	const outputDir = path.join(rootDir, 'public/docs');

	// Find all docs MDX files (excluding cloud)
	const mdxFiles = await fg(
		['**/*.mdx', '!cloud/**'],
		{ cwd: docsDir }
	);

	let count = 0;
	for (const file of mdxFiles) {
		const filePath = path.join(docsDir, file);
		const content = await readFile(filePath, 'utf-8');

		// Determine output path
		const cleanPath = file.replace(/\.mdx$/, '').replace(/\/index$/, '');
		let outputPath: string;

		if (file === 'index.mdx') {
			// Root index goes to public/docs.md
			outputPath = path.join(rootDir, 'public/docs.md');
		} else {
			outputPath = path.join(outputDir, `${cleanPath}.md`);
		}

		// Ensure directory exists
		const outputDirPath = path.dirname(outputPath);
		if (!existsSync(outputDirPath)) {
			await mkdir(outputDirPath, { recursive: true });
		}

		// Write markdown file
		await writeFile(outputPath, content, 'utf-8');
		count++;
	}

	logger.info(`Generated ${count} markdown files in public/docs/`);
}

export default generateRoutes;
