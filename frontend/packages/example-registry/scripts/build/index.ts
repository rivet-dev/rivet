// NOTE: When modifying template options or validation rules, make sure to update
// the documentation at website/src/content/docs/meta/submit-template.mdx

import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { generateScreenshots, type ScreenshotOptions } from "./screenshots.js";
import { validateReadmeFormat, validateRivetKitVersions, validatePackageJson, validateTurboJson, validateFrontendConfig } from "./validate.js";
// import { loadRailwayConfig, syncRailwayTemplate, type ExampleData } from "./railway.js";
import { TECHNOLOGIES, TAGS, type Technology, type Tag } from "../../src/const.js";

// Parse command-line arguments
function parseArgs(): { screenshotsOnly?: boolean; screenshotOptions: ScreenshotOptions } {
	const args = process.argv.slice(2);
	const screenshotOptions: ScreenshotOptions = {};
	let screenshotsOnly = false;

	for (let i = 0; i < args.length; i++) {
		const arg = args[i];
		if (arg === '--example' || arg === '-e') {
			screenshotOptions.singleExample = args[++i];
		} else if (arg === '--timeout' || arg === '-t') {
			screenshotOptions.timeout = parseInt(args[++i], 10);
		} else if (arg === '--skip-build') {
			screenshotOptions.skipBuild = true;
		} else if (arg === '--screenshots-only') {
			screenshotsOnly = true;
		} else if (arg === '--help' || arg === '-h') {
			console.log(`Usage: build [options]

Options:
  -e, --example <name>    Only generate screenshot for this example
  -t, --timeout <ms>      Timeout in milliseconds for dev server (default: 60000)
  --skip-build            Skip Docker image build (use existing)
  --screenshots-only      Only run screenshot generation, skip template processing
  -h, --help              Show this help message
`);
			process.exit(0);
		}
	}

	return { screenshotsOnly, screenshotOptions };
}

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// Extract allowed values from const definitions
const ALLOWED_TECHNOLOGIES = TECHNOLOGIES.map((t) => t.name);
const ALLOWED_TAGS = TAGS.map((t) => t.name);

interface TemplateMetadata {
	technologies: string[];
	tags: string[];
	noFrontend?: boolean;
	priority?: number;
}

interface PackageJson {
	name: string;
	license?: string;
	template?: TemplateMetadata;
	scripts?: Record<string, string>;
	dependencies?: Record<string, string>;
	devDependencies?: Record<string, string>;
}

interface Template {
	name: string;
	displayName: string;
	description: string;
	technologies: string[];
	tags: string[];
	noFrontend: boolean;
	priority?: number;
	providers: {
		vercel?: {
			name: string;
			deployUrl: string;
		};
	}
}

const VERCEL_SUFFIX = "-vercel";

function generateVercelDeployUrl(exampleName: string): string {
	const repoUrl = encodeURIComponent(
		`https://github.com/rivet-gg/rivet/tree/main/examples/${exampleName}${VERCEL_SUFFIX}`
	);
	const projectName = encodeURIComponent(`${exampleName}${VERCEL_SUFFIX}`);
	return `https://vercel.com/new/clone?repository-url=${repoUrl}&project-name=${projectName}`;
}

function validateTechnologiesAndTags(
	metadata: TemplateMetadata,
	exampleName: string,
): void {
	// Validate technologies
	for (const tech of metadata.technologies) {
		if (!ALLOWED_TECHNOLOGIES.includes(tech as any)) {
			throw new Error(
				`Invalid technology "${tech}" in ${exampleName}/package.json. Allowed technologies are: ${ALLOWED_TECHNOLOGIES.join(", ")}`,
			);
		}
	}

	// Validate tags
	for (const tag of metadata.tags) {
		if (!ALLOWED_TAGS.includes(tag as any)) {
			throw new Error(
				`Invalid tag "${tag}" in ${exampleName}/package.json. Allowed tags are: ${ALLOWED_TAGS.join(", ")}`,
			);
		}
	}
}

function generateGettingStartedSection(exampleName: string): string {
	return `## Getting Started

\`\`\`sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/${exampleName}
npm install
npm run dev
\`\`\`
`;
}

function generateLicenseSection(): string {
	return `## License

MIT
`;
}

function processReadme(content: string, exampleName: string): string {
	const lines = content.split('\n');
	let titleLine = '';
	let descriptionLine = '';
	let contentLines: string[] = [];
	let foundTitle = false;
	let foundDescription = false;
	let inGettingStarted = false;
	let inLicense = false;

	for (let i = 0; i < lines.length; i++) {
		const line = lines[i];

		// Extract title
		if (!foundTitle && line.startsWith('# ')) {
			titleLine = line;
			foundTitle = true;
			continue;
		}

		// Extract description (first non-empty line after title)
		if (foundTitle && !foundDescription && line.trim() !== '') {
			descriptionLine = line;
			foundDescription = true;
			continue;
		}

		// Skip empty lines between title and description
		if (foundTitle && !foundDescription && line.trim() === '') {
			continue;
		}

		// Skip Getting Started section
		if (line.match(/^##\s+Getting Started/i)) {
			inGettingStarted = true;
			continue;
		}

		// Skip License section
		if (line.match(/^##\s+License/i)) {
			inLicense = true;
			continue;
		}

		// Check if we're exiting a section
		if ((inGettingStarted || inLicense) && line.startsWith('## ')) {
			inGettingStarted = false;
			inLicense = false;
			// Don't skip this line, it's a new section
		}

		// Skip content within Getting Started or License sections
		if (inGettingStarted || inLicense) {
			continue;
		}

		// Add to content if we've found description
		if (foundDescription) {
			contentLines.push(line);
		}
	}

	// Reconstruct README
	const parts = [
		titleLine,
		'',
		descriptionLine,
		'',
		generateGettingStartedSection(exampleName),
		...contentLines,
	];

	// Add license at the end (trim trailing whitespace first)
	while (parts.length > 0 && parts[parts.length - 1].trim() === '') {
		parts.pop();
	}
	parts.push('');
	parts.push(generateLicenseSection().trim());

	return parts.join('\n') + '\n';
}

async function main() {
	const { screenshotsOnly, screenshotOptions } = parseArgs();

	// Path to examples directory (from example-registry package)
	const examplesDir = path.join(__dirname, "../../../../../examples");
	const outputFile = path.join(__dirname, "../../src/_gen.ts");
	const websitePublicDir = path.join(
		__dirname,
		"../../../../../website/public/examples",
	);

	// If screenshots-only mode, skip everything else
	if (screenshotsOnly) {
		await generateScreenshots(examplesDir, websitePublicDir, screenshotOptions);
		return;
	}

	// Read all example directories
	const entries = await fs.readdir(examplesDir, { withFileTypes: true });
	const exampleDirs = entries.filter((entry) => entry.isDirectory());

	const templates: Template[] = [];
	const examplesData: ExampleData[] = [];
	const errors: Array<{ example: string; error: Error }> = [];

	// Collect all directory names for Vercel variant detection
	const allDirNames = new Set(exampleDirs.map((d) => d.name));

	for (const dir of exampleDirs) {
		// Skip Vercel variant examples (they'll be linked from origin examples)
		if (dir.name.endsWith(VERCEL_SUFFIX)) {
			console.log(`⏭️  Skipping ${dir.name} (Vercel variant)`);
			continue;
		}

		const packageJsonPath = path.join(examplesDir, dir.name, "package.json");
		const readmePath = path.join(examplesDir, dir.name, "README.md");

		// Skip directories without package.json
		try {
			await fs.access(packageJsonPath);
		} catch {
			console.warn(`Skipping ${dir.name}: no package.json found`);
			continue;
		}

		try {
			// Read package.json
			const packageJsonContent = await fs.readFile(packageJsonPath, "utf-8");
			const packageJson: PackageJson = JSON.parse(packageJsonContent);

			// Error if no template metadata
			if (!packageJson.template) {
				throw new Error(
					`Missing template metadata in ${dir.name}/package.json. Please add a "template" property with technologies and tags.`,
				);
			}

			// Validate technologies and tags
			validateTechnologiesAndTags(packageJson.template, dir.name);

			// Validate package.json requirements
			validatePackageJson(packageJson, dir.name);

			// Validate rivetkit package versions
			validateRivetKitVersions(packageJson, dir.name);

			// Validate turbo.json exists
			await validateTurboJson(path.join(examplesDir, dir.name), dir.name);

			// Validate frontend configuration (must have frontendPort or noFrontend)
			validateFrontendConfig(packageJson, dir.name);

			// Read README.md to extract description and validate format
			let description = "";
			let displayName = dir.name;

			try {
				const readmeContent = await fs.readFile(readmePath, "utf-8");
				const validated = validateReadmeFormat(readmeContent, dir.name);
				displayName = validated.displayName;
				description = validated.description;

				// Process and update README with standardized getting started and license
				const processedReadme = processReadme(readmeContent, dir.name);
				await fs.writeFile(readmePath, processedReadme, "utf-8");
				console.log(`✅ Processed README for ${dir.name}`);
			} catch (error) {
				if (
					error instanceof Error &&
					error.message.includes("README format validation failed")
				) {
					throw error;
				}
				throw new Error(`Could not read README.md for ${dir.name}: ${error}`);
			}

			// Always add "rivet" as the first technology if not present
			const technologies = packageJson.template.technologies.includes("rivet")
				? packageJson.template.technologies
				: ["rivet", ...packageJson.template.technologies];

			// Check if a Vercel variant exists for this example
			const vercelVariantName = `${dir.name}${VERCEL_SUFFIX}`;
			const hasVercelVariant = allDirNames.has(vercelVariantName);

			templates.push({
				name: dir.name,
				displayName,
				description: description || `Example project for ${displayName}`,
				technologies,
				tags: packageJson.template.tags,
				noFrontend: packageJson.template.noFrontend ?? false,
				priority: packageJson.template.priority,
				providers: {
					...(hasVercelVariant && {vercel: {
							name: vercelVariantName,
							deployUrl: generateVercelDeployUrl(dir.name),
						}
					}),
				}
			});

			// Collect example data for Railway sync
			examplesData.push({
				name: dir.name,
				displayName,
				description: description || `Example project for ${displayName}`,
				readmePath,
				startCommand: packageJson.scripts?.dev || packageJson.scripts?.start,
				buildCommand: packageJson.scripts?.build,
			});
		} catch (error) {
			errors.push({
				example: dir.name,
				error: error instanceof Error ? error : new Error(String(error)),
			});
		}
	}

	// If there were any errors, print them all and exit
	if (errors.length > 0) {
		console.error("\n❌ Validation failed for the following examples:\n");
		for (const { example, error } of errors) {
			console.error(`\n${example}:`);
			console.error(`  ${error.message}`);
		}
		console.error(`\n\nTotal errors: ${errors.length}`);
		process.exit(1);
	}

	// Sort templates by priority (ascending), then by name alphabetically
	templates.sort((a, b) => {
		// Templates with priority come first
		if (a.priority !== undefined && b.priority === undefined) return -1;
		if (a.priority === undefined && b.priority !== undefined) return 1;

		// Both have priority, sort by priority value (ascending)
		if (a.priority !== undefined && b.priority !== undefined) {
			if (a.priority !== b.priority) return a.priority - b.priority;
		}

		// Same priority or both undefined, sort alphabetically by name
		return a.name.localeCompare(b.name);
	});

	// Generate TypeScript file
	const output = `// This file is auto-generated by scripts/build/index.ts
// Do not edit manually

export interface Template {
	name: string;
	displayName: string;
	description: string;
	technologies: string[];
	tags: string[];
	noFrontend: boolean;
	priority?: number;
	providers: {
		[key: string]: {
			name: string;
			deployUrl: string;
		};
	}
}

export const templates: Template[] = ${JSON.stringify(templates, null, 2)};
`;

	await fs.writeFile(outputFile, output, "utf-8");
	console.log(`✅ Generated ${templates.length} templates to ${outputFile}`);

	// Delete public images for noFrontend templates
	for (const template of templates) {
		if (template.noFrontend) {
			const imageDir = path.join(websitePublicDir, template.name);
			try {
				await fs.rm(imageDir, { recursive: true });
				console.log(`🗑️  Deleted image directory for ${template.name} (noFrontend)`);
			} catch {
				// Directory doesn't exist, nothing to delete
			}
		}
	}

	// Generate screenshots
	await generateScreenshots(examplesDir, websitePublicDir, screenshotOptions);

	// // Sync Railway templates
	// const railwayConfig = await loadRailwayConfig();
	// if (railwayConfig) {
	// 	console.log("\n🚂 Syncing Railway templates...");
	// 	const railwayErrors: Array<{ example: string; error: Error }> = [];
	//
	// 	for (const example of examplesData) {
	// 		try {
	// 			await syncRailwayTemplate(example, railwayConfig);
	// 		} catch (error) {
	// 			railwayErrors.push({
	// 				example: example.name,
	// 				error: error instanceof Error ? error : new Error(String(error)),
	// 			});
	// 		}
	// 	}
	//
	// 	if (railwayErrors.length > 0) {
	// 		console.error("\n⚠️  Some Railway templates failed to sync:\n");
	// 		for (const { example, error } of railwayErrors) {
	// 			console.error(`  ${example}: ${error.message}`);
	// 		}
	// 		console.log(`\n  ${railwayErrors.length}/${examplesData.length} templates failed`);
	// 	} else {
	// 		console.log(`\n✅ All ${examplesData.length} Railway templates synced successfully`);
	// 	}
	// }
}

main().catch((error) => {
	console.error("Failed to generate templates:", error);
	process.exit(1);
});
