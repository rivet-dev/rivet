import type { AstroIntegration } from 'astro';
import { writeFile, mkdir, readFile, copyFile } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import path from 'node:path';

interface SkillVersionData {
	rivetkit: string;
}

/**
 * Astro integration that reads the rivetkit package version and copies the openapi.json
 * for use in skill generation.
 */
export function skillVersion(): AstroIntegration {
	return {
		name: 'skill-version',
		hooks: {
			'astro:config:setup': async ({ logger }) => {
				const rootDir = process.cwd();

				logger.info('Reading rivetkit package version...');

				// Read the rivetkit package.json from the monorepo
				const rivetkitPkgPath = path.join(rootDir, '../rivetkit-typescript/packages/rivetkit/package.json');

				if (!existsSync(rivetkitPkgPath)) {
					throw new Error(`rivetkit package.json not found at ${rivetkitPkgPath}`);
				}

				const pkgContent = await readFile(rivetkitPkgPath, 'utf-8');
				const pkg = JSON.parse(pkgContent);
				const version = pkg.version;

				if (!version) {
					throw new Error('rivetkit package.json does not have a version field');
				}

				logger.info(`Found rivetkit version: ${version}`);

				// Ensure generated directory exists
				const generatedDir = path.join(rootDir, 'src/generated');
				if (!existsSync(generatedDir)) {
					await mkdir(generatedDir, { recursive: true });
				}

				// Write skill-version.json
				const versionData: SkillVersionData = {
					rivetkit: version,
				};
				const versionOutputPath = path.join(generatedDir, 'skill-version.json');
				await writeFile(
					versionOutputPath,
					JSON.stringify(versionData, null, 2),
					'utf-8'
				);

				logger.info(`Generated skill-version.json with rivetkit v${version}`);

				// Copy openapi.json from rivetkit-openapi
				const openapiSrcPath = path.join(rootDir, '../rivetkit-openapi/openapi.json');
				const openapiDestPath = path.join(generatedDir, 'rivetkit-openapi.json');

				if (!existsSync(openapiSrcPath)) {
					throw new Error(`rivetkit openapi.json not found at ${openapiSrcPath}`);
				}

				await copyFile(openapiSrcPath, openapiDestPath);
				logger.info(`Copied rivetkit openapi.json to generated directory`);
			},
		},
	};
}

export default skillVersion;
