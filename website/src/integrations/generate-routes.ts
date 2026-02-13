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

interface FrontmatterData {
	title: string;
	description: string;
}

function filePathToHref(filePath: string): string {
	return '/' + filePath
		.replace(/src\/content\//, '')
		.replace(/\/index\.mdx$/, '')
		.replace(/\.mdx$/, '')
		.replace(/\/page$/, '');
}

function extractFrontmatterData(content: string): FrontmatterData {
	const frontmatterMatch = content.match(/^---\n([\s\S]*?)\n---/);
	const frontmatter = frontmatterMatch?.[1] ?? '';
	const titleMatch = frontmatter.match(/^title:\s*(.+)$/m);
	const descMatch = frontmatter.match(/^description:\s*(.+)$/m);

	return {
		title: titleMatch ? titleMatch[1].trim().replace(/^"|"$/g, '') : '',
		description: descMatch ? descMatch[1].trim().replace(/^"|"$/g, '') : '',
	};
}

function cleanMdxForMarkdown(content: string): string {
	const contentWithoutFrontmatter = content.replace(/^---\n[\s\S]*?\n---\n*/m, '');
	const lines = contentWithoutFrontmatter.split(/\r?\n/);
	const cleanedLines: string[] = [];

	let inFence = false;
	let fenceToken = '';
	let inComment = false;
	let inModuleBlock = false;
	let expressionDepth = 0;
	let scriptDepth = 0;
	let skippedComponentDepth = 0;
	const skippedComponentTags = new Set(['Card', 'CardGroup']);

	const countChar = (input: string, char: string) =>
		input.split(char).length - 1;
	const countDelta = (input: string, openChar: string, closeChar: string) =>
		countChar(input, openChar) - countChar(input, closeChar);
	const isPureTagLine = (line: string) =>
		/^\s*<\/?[A-Za-z][\w:-]*(?:\s+[^>]*)?\s*\/?>\s*$/.test(line);
	const isScriptStartLine = (line: string) =>
		/^\s*(const|let|var|function)\s+/.test(line);
	const getSkippedComponentTagDelta = (line: string): number | null => {
		const trimmed = line.trim();
		const tagMatch = trimmed.match(/^<\/?([A-Za-z][\w:-]*)(?:\s+[^>]*)?\s*\/?>$/);
		if (!tagMatch) {
			return null;
		}
		const tagName = tagMatch[1];
		if (!skippedComponentTags.has(tagName)) {
			return null;
		}
		if (trimmed.startsWith('</')) {
			return -1;
		}
		if (trimmed.endsWith('/>')) {
			return 0;
		}
		return 1;
	};

	for (const originalLine of lines) {
		let line = originalLine;
		const trimmed = line.trim();
		const fenceMatch = line.match(/^\s*(```+|~~~+)/);

		if (fenceMatch) {
			if (!inFence) {
				inFence = true;
				fenceToken = fenceMatch[1];
				line = line
					.replace(/\s+\{\{[^}]+\}\}\s*$/, '')
					.replace(/\s+\{[^}]+\}\s*$/, '');
			} else if (trimmed.startsWith(fenceToken)) {
				inFence = false;
				fenceToken = '';
			}
			cleanedLines.push(line);
			continue;
		}

		if (inFence) {
			cleanedLines.push(line);
			continue;
		}

		if (inComment) {
			if (line.includes('-->')) {
				inComment = false;
			}
			continue;
		}
		if (line.includes('<!--')) {
			if (!line.includes('-->')) {
				inComment = true;
			}
			continue;
		}

		if (inModuleBlock) {
			if (trimmed.endsWith(';')) {
				inModuleBlock = false;
			}
			continue;
		}

		if (skippedComponentDepth > 0) {
			const delta = getSkippedComponentTagDelta(line);
			if (delta !== null) {
				skippedComponentDepth = Math.max(0, skippedComponentDepth + delta);
			}
			continue;
		}

		if (expressionDepth > 0) {
			expressionDepth += countDelta(line, '{', '}');
			if (expressionDepth <= 0) {
				expressionDepth = 0;
			}
			continue;
		}
		if (scriptDepth > 0) {
			scriptDepth += countDelta(line, '{', '}');
			scriptDepth += countDelta(line, '[', ']');
			scriptDepth += countDelta(line, '(', ')');
			if (scriptDepth <= 0 && trimmed.endsWith(';')) {
				scriptDepth = 0;
			}
			continue;
		}

		if (/^\s*import\s+/.test(line) || /^\s*export\s+/.test(line)) {
			if (!trimmed.endsWith(';')) {
				inModuleBlock = true;
			}
			continue;
		}

		if (isScriptStartLine(line)) {
			scriptDepth += countDelta(line, '{', '}');
			scriptDepth += countDelta(line, '[', ']');
			scriptDepth += countDelta(line, '(', ')');
			if (scriptDepth <= 0 && trimmed.endsWith(';')) {
				scriptDepth = 0;
			} else if (scriptDepth <= 0) {
				scriptDepth = 1;
			}
			continue;
		}

		if (/^\s*\{/.test(line)) {
			expressionDepth = countDelta(line, '{', '}');
			if (expressionDepth <= 0) {
				expressionDepth = 0;
			}
			continue;
		}

		const skippedTagDelta = getSkippedComponentTagDelta(line);
		if (skippedTagDelta !== null) {
			if (skippedTagDelta > 0) {
				skippedComponentDepth = skippedTagDelta;
			}
			continue;
		}

		if (isPureTagLine(line)) {
			continue;
		}
		if (/^[\[\](){};,]+$/.test(trimmed)) {
			continue;
		}
		cleanedLines.push(line);
	}

	return cleanedLines
		.join('\n')
		.replace(/\n{3,}/g, '\n\n')
		.trim();
}

function buildMarkdownDocument(title: string, description: string, body: string): string {
	const header = description ? `# ${title}\n\n${description}` : `# ${title}`;
	return body ? `${header}\n\n${body}` : header;
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
					const { title, description } = extractFrontmatterData(content);

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

				// Generate individual markdown files in public/{docs,guides}/
				await generateMarkdownFiles(rootDir, logger);
			},
		},
	};
}

/**
 * Generate individual markdown files in public/{docs,guides}/
 */
async function generateMarkdownFiles(rootDir: string, logger: { info: (msg: string) => void; warn: (msg: string) => void }) {
	const docsCount = await generateCollectionMarkdownFiles(
		rootDir,
		'docs',
		['**/*.mdx', '!cloud/**']
	);
	const guidesCount = await generateCollectionMarkdownFiles(
		rootDir,
		'guides',
		['**/*.mdx']
	);

	logger.info(`Generated ${docsCount} markdown files in public/docs/`);
	logger.info(`Generated ${guidesCount} markdown files in public/guides/`);
}

async function generateCollectionMarkdownFiles(
	rootDir: string,
	collectionName: string,
	patterns: string[]
) {
	const collectionDir = path.join(rootDir, `src/content/${collectionName}`);
	if (!existsSync(collectionDir)) {
		return 0;
	}

	const outputDir = path.join(rootDir, `public/${collectionName}`);
	const mdxFiles = await fg(patterns, { cwd: collectionDir });

	let count = 0;
	for (const file of mdxFiles) {
		const filePath = path.join(collectionDir, file);
		const content = await readFile(filePath, 'utf-8');
		const { title, description } = extractFrontmatterData(content);
		const markdownBody = cleanMdxForMarkdown(content);
		const cleanContent = buildMarkdownDocument(title || 'Untitled', description, markdownBody);

		const cleanPath = file.replace(/\.mdx$/, '').replace(/\/index$/, '');
		const outputPath = file === 'index.mdx'
			? path.join(rootDir, `public/${collectionName}.md`)
			: path.join(outputDir, `${cleanPath}.md`);

		const outputDirPath = path.dirname(outputPath);
		if (!existsSync(outputDirPath)) {
			await mkdir(outputDirPath, { recursive: true });
		}

		await writeFile(outputPath, cleanContent, 'utf-8');
		count++;
	}

	return count;
}

export default generateRoutes;
