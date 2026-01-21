import type { AstroIntegration } from 'astro';
import { writeFile, mkdir } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import path from 'node:path';
import fg from 'fast-glob';
import { readFile } from 'node:fs/promises';

interface PageData {
	title: string;
	description: string | null;
}

interface RoutesData {
	pages: Record<string, PageData>;
}

function filePathToHref(filePath: string): string {
	return '/' + filePath
		.replace(/src\/content\//, '')
		.replace(/\/index\.mdx$/, '')
		.replace(/\.mdx$/, '')
		.replace(/\/page$/, '');
}

/**
 * Astro integration that generates routes.json at build time
 */
export function generateRoutes(): AstroIntegration {
	return {
		name: 'generate-routes',
		hooks: {
			'astro:config:setup': async ({ logger }) => {
				const rootDir = process.cwd();
				const pages: Record<string, PageData> = {};

				logger.info('Generating routes.json...');

				const mdxFiles = await fg(['src/content/**/*.mdx'], { cwd: rootDir });
				for (const file of mdxFiles) {
					const filePath = path.join(rootDir, file);
					const content = await readFile(filePath, 'utf-8');
					const frontmatterMatch = content.match(/^---\n([\s\S]*?)\n---/);
					const frontmatter = frontmatterMatch?.[1] ?? '';
					const titleMatch = frontmatter.match(/^title:\s*(.+)$/m);
					const descMatch = frontmatter.match(/^description:\s*(.+)$/m);
					const title = titleMatch ? titleMatch[1].trim().replace(/^"|"$/g, '') : '';
					const description = descMatch ? descMatch[1].trim().replace(/^"|"$/g, '') : '';

					if (!title) {
						logger.warn(`Missing title in ${file}`);
					}

					const href = filePathToHref(file);
					pages[href] = {
						title: title || 'Untitled',
						description: description || '',
					};
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

		const frontmatterMatch = content.match(/^---\n([\s\S]*?)\n---/);
		const frontmatter = frontmatterMatch?.[1] ?? '';
		const titleMatch = frontmatter.match(/^title:\s*(.+)$/m);
		const descMatch = frontmatter.match(/^description:\s*(.+)$/m);
		const title = titleMatch ? titleMatch[1].trim().replace(/^"|"$/g, '') : 'Untitled';
		const description = descMatch ? descMatch[1].trim().replace(/^"|"$/g, '') : '';
		const cleanContent = description ? `# ${title}\n\n${description}` : `# ${title}`;

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
		await writeFile(outputPath, cleanContent, 'utf-8');
		count++;
	}

	logger.info(`Generated ${count} markdown files in public/docs/`);
}

export default generateRoutes;
