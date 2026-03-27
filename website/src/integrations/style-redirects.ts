import type { AstroIntegration } from 'astro';
import { readFile, writeFile } from 'node:fs/promises';
import fg from 'fast-glob';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

/**
 * Astro integration that adds dark background styling to redirect HTML files
 * generated during static builds. Without this, redirect pages flash white
 * before the browser follows the meta refresh.
 */
export function styleRedirects(): AstroIntegration {
	return {
		name: 'style-redirects',
		hooks: {
			'astro:build:done': async ({ dir, logger }) => {
				const outDir = fileURLToPath(dir);
				const htmlFiles = await fg(['**/*.html'], { cwd: outDir });

				let count = 0;
				for (const file of htmlFiles) {
					const filePath = path.join(outDir, file);
					const content = await readFile(filePath, 'utf-8');

					// Only process redirect pages (contain meta refresh but no full page body)
					if (!content.includes('http-equiv="refresh"') && !content.includes("http-equiv='refresh'")) {
						continue;
					}

					// Inject dark background style into the redirect HTML
					const styled = content.replace(
						'<head>',
						'<head><style>html{background-color:#0c0a09}</style>'
					);

					if (styled !== content) {
						await writeFile(filePath, styled, 'utf-8');
						count++;
					}
				}

				if (count > 0) {
					logger.info(`Styled ${count} redirect pages with dark background`);
				}
			},
		},
	};
}

export default styleRedirects;
