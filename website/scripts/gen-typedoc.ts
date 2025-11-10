#!/usr/bin/env tsx
import * as fs from "node:fs/promises";
import { resolve, dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { exec } from "node:child_process";
import { promisify } from "node:util";

const execAsync = promisify(exec);
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

interface PackageJson {
	name?: string;
	private?: boolean;
	workspaces?: string[] | { packages?: string[] };
	exports?: any;
}

interface PackageInfo {
	name: string;
	path: string;
	entryPoints: string[];
}

/**
 * Recursively extract all .d.ts file paths from package.json exports
 */
function extractDtsFiles(exports: any, result: string[] = []): string[] {
	if (typeof exports === "string") {
		if (exports.endsWith(".d.ts")) {
			result.push(exports);
		}
	} else if (typeof exports === "object" && exports !== null) {
		for (const value of Object.values(exports)) {
			extractDtsFiles(value, result);
		}
	}
	return result;
}

/**
 * Map a .d.ts path to its corresponding source .ts file
 * Example: ./dist/tsup/mod.d.ts -> ./src/mod.ts
 * Example: ./dist/mod.d.ts -> ./src/mod.ts
 * Example: ./dist/drizzle/mod.d.ts -> ./src/drizzle/mod.ts
 */
function mapDtsToSource(dtsPath: string): string {
	// Remove leading ./
	let path = dtsPath.replace(/^\.\//, "");

	// Replace dist directory with src, preserving subdirectories
	// Common patterns: dist/tsup/, dist/, dist/esm/, etc.
	// For dist/drizzle/mod.d.ts -> src/drizzle/mod.ts
	if (path.startsWith("dist/")) {
		// Remove dist/ prefix
		path = path.replace(/^dist\//, "");
		// Remove build tool directories (tsup, esm, cjs, etc) if they're the first segment
		path = path.replace(/^(?:tsup|esm|cjs|umd|iife)\//, "");
		// Add src/ prefix
		path = `src/${path}`;
	}

	// Replace .d.ts with .ts
	path = path.replace(/\.d\.ts$/, ".ts");

	return `./${path}`;
}

/**
 * Check if a file exists and is a file (not directory)
 */
async function fileExists(filePath: string): Promise<boolean> {
	try {
		const stats = await fs.stat(filePath);
		return stats.isFile();
	} catch {
		return false;
	}
}

/**
 * Get entry points from package exports, preferring source .ts files
 */
async function getEntryPoints(
	packagePath: string,
	packageName: string,
	exports: any,
): Promise<string[]> {
	const dtsFiles = extractDtsFiles(exports);
	const uniqueDtsFiles = [...new Set(dtsFiles)];
	const entryPointsMap = new Map<string, string>();

	console.log(`  Found ${uniqueDtsFiles.length} unique .d.ts files in exports`);

	// Force .d.ts only for engine-api-full due to source compilation errors
	const preferDts = packageName === "@rivetkit/engine-api-full";

	for (const dtsFile of uniqueDtsFiles) {
		const sourcePath = mapDtsToSource(dtsFile);
		const sourceFullPath = resolve(packagePath, sourcePath);
		const dtsFullPath = resolve(packagePath, dtsFile);

		// Try source file first (unless preferDts is true)
		if (!preferDts && (await fileExists(sourceFullPath))) {
			console.log(`    ✓ Using source: ${sourcePath}`);
			entryPointsMap.set(sourcePath, sourcePath);
		} else if (await fileExists(dtsFullPath)) {
			console.log(`    ⚠ Falling back to .d.ts: ${dtsFile}`);
			entryPointsMap.set(dtsFile, dtsFile);
		} else {
			console.log(`    ✗ Neither source nor .d.ts found for: ${dtsFile}`);
		}
	}

	return Array.from(entryPointsMap.values());
}

/**
 * Create a TypeDoc config file for the package
 */
async function createTypeDocConfig(
	packagePath: string,
	entryPoints: string[],
): Promise<void> {
	const config = {
		entryPoints,
		tsconfig: "./tsconfig.typedoc.json",
	};

	const configPath = resolve(packagePath, "typedoc.json");
	await fs.writeFile(configPath, JSON.stringify(config, null, 2));
	console.log(`  Created typedoc.json`);
}

/**
 * Create a tsconfig that extends the package's base tsconfig
 */
async function createTypedocTsconfig(
	packagePath: string,
	packageName: string,
	entryPoints: string[],
): Promise<void> {
	// Check if package has a tsconfig.json
	const baseTsconfigPath = resolve(packagePath, "tsconfig.json");
	let extendsPath = "./tsconfig.json";

	try {
		await fs.access(baseTsconfigPath);
	} catch {
		// No base tsconfig, don't extend anything
		extendsPath = "../../../tsconfig.base.json";
	}

	// Determine include paths based on entry points
	// If all entry points are .d.ts files, only include those directories
	const allDts = entryPoints.every((ep) => ep.endsWith(".d.ts"));
	let include: string[];

	if (allDts) {
		// Extract unique directories from .d.ts entry points
		const dirs = new Set<string>();
		for (const ep of entryPoints) {
			const dir = dirname(ep.replace(/^\.\//, ""));
			dirs.add(`${dir}/**/*`);
		}
		include = Array.from(dirs);
		console.log(`    Using .d.ts-only includes: ${include.join(", ")}`);
	} else {
		// Include both src and dist for source files
		include = ["src/**/*", "dist/**/*"];
	}

	const config = {
		extends: extendsPath,
		compilerOptions: {
			skipLibCheck: true,
		},
		include,
	};

	const configPath = resolve(packagePath, "tsconfig.typedoc.json");
	await fs.writeFile(configPath, JSON.stringify(config, null, 2));
	console.log(`  Created tsconfig.typedoc.json`);
}

/**
 * Find all TypeScript packages in the workspace and extract their entry points
 */
async function findTypeScriptPackages(
	workspaceRoot: string,
): Promise<PackageInfo[]> {
	try {
		const { stdout } = await execAsync("pnpm list -r --depth -1 --json", {
			cwd: workspaceRoot,
		});

		const packages = JSON.parse(stdout) as Array<{
			name: string;
			path: string;
			private: boolean;
		}>;

		const tsPackages: PackageInfo[] = [];

		for (const pkg of packages) {
			// Only include rivetkit packages for documentation
			if (!pkg.name.startsWith("@rivetkit/") && pkg.name !== "rivetkit") {
				continue;
			}

			// Read package.json to check if it has exports
			const pkgJsonPath = resolve(pkg.path, "package.json");
			try {
				const pkgJsonContent = await fs.readFile(pkgJsonPath, "utf-8");
				const pkgJson: PackageJson = JSON.parse(pkgJsonContent);

				// Only include if it has exports (public API)
				if (pkgJson.exports) {
					console.log(`\nProcessing ${pkg.name}:`);

					const entryPoints = await getEntryPoints(
						pkg.path,
						pkg.name,
						pkgJson.exports,
					);

					if (entryPoints.length > 0) {
						await createTypeDocConfig(pkg.path, entryPoints);
						await createTypedocTsconfig(pkg.path, pkg.name, entryPoints);

						tsPackages.push({
							name: pkg.name,
							path: pkg.path,
							entryPoints,
						});

						console.log(`  ✓ Package ready with ${entryPoints.length} entry points`);
					} else {
						console.log(`  ✗ No valid entry points found, skipping`);
					}
				}
			} catch (error) {
				console.error(`  ✗ Error processing package:`, error);
			}
		}

		return tsPackages;
	} catch (error: any) {
		console.error("Failed to list workspace packages:", error.message);
		return [];
	}
}

/**
 * Clean up generated config files
 */
async function cleanupConfigs(packages: PackageInfo[]): Promise<void> {
	console.log("\nCleaning up generated config files...");
	for (const pkg of packages) {
		try {
			await fs.unlink(resolve(pkg.path, "typedoc.json"));
			await fs.unlink(resolve(pkg.path, "tsconfig.typedoc.json"));
		} catch (error) {
			// Ignore errors
		}
	}
}

async function main() {
	const websiteRoot = resolve(__dirname, "..");
	const workspaceRoot = resolve(websiteRoot, "..");
	const outputPath = resolve(websiteRoot, "public", "typedoc");

	console.log("Website root:", websiteRoot);
	console.log("Workspace root:", workspaceRoot);
	console.log("Output path:", outputPath);

	// Find all TypeScript packages in the workspace
	console.log("\nSearching for TypeScript packages in workspace...");
	const packages = await findTypeScriptPackages(workspaceRoot);

	if (packages.length === 0) {
		console.error("\nNo TypeScript packages with exports found!");
		process.exit(1);
	}

	console.log(`\n✓ Found ${packages.length} packages ready for documentation`);

	// Clean up old documentation
	console.log("\nCleaning up old documentation...");
	try {
		await fs.rm(outputPath, { recursive: true, force: true });
	} catch (error) {
		// Ignore errors if directory doesn't exist
	}

	// Ensure output directory exists
	await fs.mkdir(outputPath, { recursive: true });

	console.log(`\nGenerating TypeDoc for ${packages.length} packages...`);

	// Run typedoc with packages entry point strategy
	const packagePathsArg = packages.map((p) => p.path).join(" ");

	try {
		const { stdout, stderr } = await execAsync(
			`NODE_OPTIONS="--max-old-space-size=8192" pnpm exec typedoc ${packagePathsArg} --out ${outputPath} --entryPointStrategy packages --readme none --hideGenerator --excludeExternals --excludePrivate`,
			{
				cwd: websiteRoot,
			},
		);

		if (stdout) console.log(stdout);
		if (stderr) console.error(stderr);

		console.log(`\n✓ TypeDoc generated successfully at ${outputPath}`);
	} catch (error: any) {
		console.error(`✗ Failed to generate TypeDoc:`, error.message);
		if (error.stdout) console.log(error.stdout);
		if (error.stderr) console.error(error.stderr);
		process.exit(1);
	} finally {
		// Clean up generated config files
		await cleanupConfigs(packages);
	}
}

main();
