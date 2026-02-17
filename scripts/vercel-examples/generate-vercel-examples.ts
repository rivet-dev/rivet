#!/usr/bin/env npx tsx

/**
 * Generates Vercel-flavored versions of examples.
 *
 * This script:
 * 1. Detects changes in origin examples using git diff
 * 2. Generates Vercel-compatible versions at examples/{name}-vercel/
 * 3. Creates api/index.ts that exports the Hono app with handle() adapter
 * 4. Creates appropriate vercel.json configuration
 *
 * Usage:
 *   npx tsx scripts/vercel-examples/generate-vercel-examples.ts
 *   npx tsx scripts/vercel-examples/generate-vercel-examples.ts --example hello-world
 *   npx tsx scripts/vercel-examples/generate-vercel-examples.ts --force
 *   npx tsx scripts/vercel-examples/generate-vercel-examples.ts --all
 */

import { execSync } from "node:child_process";
import * as fs from "node:fs";
import * as path from "node:path";

const EXAMPLES_DIR = path.resolve(import.meta.dirname, "../../examples");
const VERCEL_SUFFIX = "-vercel";

interface ExampleConfig {
	name: string;
	dir: string;
	hasFrontend: boolean;
	packageJson: PackageJson;
}

interface PackageJson {
	name: string;
	version: string;
	type?: string;
	scripts?: Record<string, string>;
	dependencies?: Record<string, string>;
	devDependencies?: Record<string, string>;
	noFrontend?: boolean;
	frontendPort?: number;
	skipVercel?: boolean;
	[key: string]: unknown;
}

interface CliArgs {
	example?: string;
	force: boolean;
	all: boolean;
	dryRun: boolean;
}

function parseArgs(): CliArgs {
	const args = process.argv.slice(2);
	const result: CliArgs = {
		force: false,
		all: false,
		dryRun: false,
	};

	for (let i = 0; i < args.length; i++) {
		if (args[i] === "--example" && args[i + 1]) {
			result.example = args[++i];
		} else if (args[i] === "--force") {
			result.force = true;
		} else if (args[i] === "--all") {
			result.all = true;
		} else if (args[i] === "--dry-run") {
			result.dryRun = true;
		}
	}

	return result;
}

function getExamples(): ExampleConfig[] {
	const examples: ExampleConfig[] = [];
	const entries = fs.readdirSync(EXAMPLES_DIR, { withFileTypes: true });

	for (const entry of entries) {
		if (!entry.isDirectory()) continue;
		// Skip vercel examples and special directories
		if (entry.name.endsWith(VERCEL_SUFFIX)) continue;
		if (entry.name.startsWith(".")) continue;

		const packageJsonPath = path.join(EXAMPLES_DIR, entry.name, "package.json");
		if (!fs.existsSync(packageJsonPath)) continue;

		try {
			const packageJson = JSON.parse(
				fs.readFileSync(packageJsonPath, "utf-8")
			) as PackageJson;
			const hasFrontend = !(packageJson.noFrontend ?? false);

			examples.push({
				name: entry.name,
				dir: path.join(EXAMPLES_DIR, entry.name),
				hasFrontend,
				packageJson,
			});
		} catch {
			// Skip invalid package.json files
		}
	}

	return examples;
}

function getChangedExamples(): Set<string> {
	const changed = new Set<string>();

	try {
		// Check for staged changes
		const stagedOutput = execSync("git diff --cached --name-only", {
			encoding: "utf-8",
			cwd: EXAMPLES_DIR,
		});

		// Check for unstaged changes
		const unstagedOutput = execSync("git diff --name-only", {
			encoding: "utf-8",
			cwd: EXAMPLES_DIR,
		});

		// Check for untracked files
		const untrackedOutput = execSync(
			"git ls-files --others --exclude-standard",
			{
				encoding: "utf-8",
				cwd: EXAMPLES_DIR,
			}
		);

		const allChanges = [
			...stagedOutput.split("\n"),
			...unstagedOutput.split("\n"),
			...untrackedOutput.split("\n"),
		].filter(Boolean);

		for (const file of allChanges) {
			// Extract example name from path like "examples/hello-world/src/server.ts"
			const match = file.match(/^examples\/([^/]+)\//);
			if (match) {
				const exampleName = match[1];
				// Skip vercel examples
				if (!exampleName.endsWith(VERCEL_SUFFIX)) {
					changed.add(exampleName);
				}
			}
		}
	} catch {
		console.warn("Warning: Could not detect git changes, assuming all changed");
		return new Set(["*"]);
	}

	return changed;
}

function shouldSkipExample(example: ExampleConfig): string | null {
	// Skip if explicitly marked to skip Vercel generation
	if (example.packageJson.skipVercel) {
		return "Marked to skip Vercel generation";
	}

	// Skip next.js examples - they have their own deployment strategy
	if (example.name.includes("next-js") || example.name.includes("nextjs")) {
		return "Next.js has its own Vercel integration";
	}

	// Skip cloudflare examples - different runtime
	if (example.name.includes("cloudflare")) {
		return "Cloudflare Workers have different runtime";
	}

	// Skip deno examples - different runtime
	if (example.name === "deno") {
		return "Deno has different runtime";
	}

	// Check if src/server.ts exists
	const serverPath = path.join(example.dir, "src", "server.ts");
	if (!fs.existsSync(serverPath)) {
		return "No src/server.ts found";
	}

	return null;
}

function generateVercelJson(example: ExampleConfig): object {
	if (example.hasFrontend) {
		// Frontend + API: Use vite for frontend, api route for backend
		return {
			framework: "vite",
			rewrites: [{ source: "/api/(.*)", destination: "/api" }],
		};
	}
	// API only
	return {
		rewrites: [{ source: "/(.*)", destination: "/api" }],
	};
}

function generateApiIndexTs(example: ExampleConfig): string {
	return `import app from "../src/server.ts";

export default app;
`;
}

function generateVercelDeployUrl(exampleName: string): string {
	const repoUrl = encodeURIComponent(
		`https://github.com/rivet-gg/rivet/tree/main/examples/${exampleName}${VERCEL_SUFFIX}`
	);
	const projectName = encodeURIComponent(`${exampleName}${VERCEL_SUFFIX}`);
	return `https://vercel.com/new/clone?repository-url=${repoUrl}&project-name=${projectName}`;
}

function generatePackageJson(example: ExampleConfig): PackageJson {
	const original = example.packageJson;
	const newPkg: PackageJson = {
		name: `${original.name}${VERCEL_SUFFIX}`,
		version: original.version,
		private: true,
		type: "module",
		scripts: {},
		dependencies: { ...original.dependencies },
		devDependencies: { ...original.devDependencies },
		stableVersion: original.stableVersion as string | undefined,
		license: original.license as string | undefined,
	};

	// Update scripts for Vercel
	if (example.hasFrontend) {
		newPkg.scripts = {
			dev: "vercel dev",
			build: "vite build",
			"check-types": "tsc --noEmit",
		};
	} else {
		newPkg.scripts = {
			dev: "vercel dev",
			"check-types": "tsc --noEmit",
		};
	}

	// Remove srvx-related dependencies (not needed for Vercel)
	delete newPkg.dependencies?.["srvx"];
	delete newPkg.devDependencies?.["vite-plugin-srvx"];

	return newPkg;
}

function generateViteConfig(): string {
	return `import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
	plugins: [react()],
});
`;
}

function generateTsConfig(example: ExampleConfig): object {
	const includeList = ["src/**/*", "api/**/*"];
	if (example.hasFrontend) {
		includeList.push("frontend/**/*");
	}

	return {
		compilerOptions: {
			target: "esnext",
			lib: example.hasFrontend ? ["esnext", "dom", "dom.iterable"] : ["esnext"],
			jsx: example.hasFrontend ? "react-jsx" : undefined,
			module: "esnext",
			moduleResolution: "bundler",
			types: example.hasFrontend ? ["node", "vite/client"] : ["node"],
			noEmit: true,
			strict: true,
			skipLibCheck: true,
			allowImportingTsExtensions: true,
			rewriteRelativeImportExtensions: true,
		},
		include: includeList,
	};
}

function copyDirectory(src: string, dest: string, exclude: string[] = []): void {
	if (!fs.existsSync(dest)) {
		fs.mkdirSync(dest, { recursive: true });
	}

	const entries = fs.readdirSync(src, { withFileTypes: true });

	for (const entry of entries) {
		if (exclude.includes(entry.name)) continue;

		const srcPath = path.join(src, entry.name);
		const destPath = path.join(dest, entry.name);

		if (entry.isDirectory()) {
			copyDirectory(srcPath, destPath, exclude);
		} else {
			fs.copyFileSync(srcPath, destPath);
		}
	}
}

function generateVercelExample(
	example: ExampleConfig,
	dryRun: boolean
): void {
	const vercelDir = path.join(EXAMPLES_DIR, `${example.name}${VERCEL_SUFFIX}`);

	console.log(`  📁 Output: ${vercelDir}`);

	if (dryRun) {
		console.log(`  🔍 [DRY RUN] Would generate Vercel example`);
		return;
	}

	// Remove existing vercel example if it exists
	if (fs.existsSync(vercelDir)) {
		fs.rmSync(vercelDir, { recursive: true });
	}

	// Create the vercel example directory
	fs.mkdirSync(vercelDir, { recursive: true });

	// Copy source files, excluding certain files/directories
	const excludeList = [
		"node_modules",
		".actorcore",
		"dist",
		".turbo",
		".vercel",
		"vercel.json",
		"package.json",
		"tsconfig.json",
		"vite.config.ts",
		"turbo.json",
	];

	copyDirectory(example.dir, vercelDir, excludeList);

	// Create api/index.ts
	const apiDir = path.join(vercelDir, "api");
	fs.mkdirSync(apiDir, { recursive: true });
	fs.writeFileSync(
		path.join(apiDir, "index.ts"),
		generateApiIndexTs(example)
	);

	// Create vercel.json
	fs.writeFileSync(
		path.join(vercelDir, "vercel.json"),
		JSON.stringify(generateVercelJson(example), null, "\t") + "\n"
	);

	// Create package.json
	fs.writeFileSync(
		path.join(vercelDir, "package.json"),
		JSON.stringify(generatePackageJson(example), null, "\t") + "\n"
	);

	// Create tsconfig.json
	fs.writeFileSync(
		path.join(vercelDir, "tsconfig.json"),
		JSON.stringify(generateTsConfig(example), null, "\t") + "\n"
	);

	// Create vite.config.ts for frontend examples
	if (example.hasFrontend) {
		fs.writeFileSync(
			path.join(vercelDir, "vite.config.ts"),
			generateViteConfig()
		);
	}

	// Create turbo.json
	fs.writeFileSync(
		path.join(vercelDir, "turbo.json"),
		JSON.stringify(
			{
				$schema: "https://turbo.build/schema.json",
				extends: ["//"],
			},
			null,
			"\t"
		) + "\n"
	);

	// Create .gitignore
	fs.writeFileSync(
		path.join(vercelDir, ".gitignore"),
		".actorcore\nnode_modules\ndist\n.vercel\n"
	);

	// Update README if it exists
	const readmePath = path.join(vercelDir, "README.md");
	if (fs.existsSync(readmePath)) {
		let readme = fs.readFileSync(readmePath, "utf-8");
		// Add Vercel-specific note and deploy button at the top
		if (!readme.includes("Vercel-optimized")) {
			const deployUrl = generateVercelDeployUrl(example.name);
			const vercelNote = `> **Note:** This is the Vercel-optimized version of the [${example.name}](../${example.name}) example.
> It uses the \`hono/vercel\` adapter and is configured for Vercel deployment.

[![Deploy with Vercel](https://vercel.com/button)](${deployUrl})

`;
			readme = vercelNote + readme;
			fs.writeFileSync(readmePath, readme);
		}
	}

	console.log(`  ✅ Generated successfully`);
}

async function main(): Promise<void> {
	const args = parseArgs();
	let examples = getExamples();

	console.log(`\n🔍 Found ${examples.length} origin examples\n`);

	// Filter by specific example if provided
	if (args.example) {
		examples = examples.filter((e) => e.name === args.example);
		if (examples.length === 0) {
			console.error(`❌ Example "${args.example}" not found`);
			process.exit(1);
		}
	}

	// Determine which examples need regeneration
	let examplesToGenerate: ExampleConfig[] = [];

	if (args.force || args.all) {
		examplesToGenerate = examples;
		console.log(`📦 Generating all ${examplesToGenerate.length} examples (--force/--all)\n`);
	} else {
		const changedExamples = getChangedExamples();

		if (changedExamples.has("*")) {
			examplesToGenerate = examples;
			console.log(`📦 Could not detect changes, generating all examples\n`);
		} else if (changedExamples.size === 0) {
			console.log(`✅ No changes detected in origin examples\n`);
			return;
		} else {
			examplesToGenerate = examples.filter((e) => changedExamples.has(e.name));
			console.log(
				`📦 Detected changes in ${changedExamples.size} examples: ${[...changedExamples].join(", ")}\n`
			);
		}
	}

	// Generate Vercel examples
	let generated = 0;
	let skipped = 0;

	for (const example of examplesToGenerate) {
		console.log(`\n🔧 Processing: ${example.name}`);

		const skipReason = shouldSkipExample(example);
		if (skipReason) {
			console.log(`  ⏭️  Skipping: ${skipReason}`);
			skipped++;
			continue;
		}

		generateVercelExample(example, args.dryRun);
		generated++;
	}

	// Print summary
	console.log("\n" + "=".repeat(60));
	console.log("📊 SUMMARY");
	console.log("=".repeat(60));
	console.log(`\n✅ Generated: ${generated}`);
	console.log(`⏭️  Skipped: ${skipped}`);

	if (args.dryRun) {
		console.log(`\n🔍 This was a dry run. No files were modified.`);
	}

	console.log("\n");
}

main().catch((err) => {
	console.error("Fatal error:", err);
	process.exit(1);
});
