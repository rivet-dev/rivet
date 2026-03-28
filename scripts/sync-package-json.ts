#!/usr/bin/env tsx

/**
 * Reads the "rivetkit" entry map from each package.json and generates:
 * - exports map
 * - build script
 * - standard fields (type, engines, sideEffects, files)
 *
 * The "rivetkit" block maps export paths to source files:
 *
 *   "rivetkit": {
 *     ".": "src/mod.ts",
 *     "./client": "src/client/mod.ts"
 *   }
 *
 * Browser entries use an object:
 *
 *   "./client": { "default": "src/client/mod.ts", "browser": "src/client/mod.browser.ts" }
 *
 * Config options (keys not starting with "."):
 *   - outDir: JS output directory (default: "dist")
 *   - typesDir: types output directory (default: "dist/types")
 *   - preBuild: command to run before tsup
 *   - postBuild: command to run after tsc-alias
 *   - browserOutDir: browser build output directory (default: "dist/browser")
 *   - rootDir: tsconfig.build.json rootDir (default: "src")
 *   - extraIncludes: additional include globs for tsconfig.build.json
 *   - extraExcludes: additional exclude globs for tsconfig.build.json
 *   - static entries: { "static": "path/to/file" }
 *
 * Usage:
 *   tsx scripts/sync-package-json.ts          # apply changes
 *   tsx scripts/sync-package-json.ts --check  # exit non-zero if out of sync
 */

import * as fs from "node:fs";
import * as path from "node:path";

const ROOT = path.resolve(new URL(".", import.meta.url).pathname, "..");

const PACKAGE_DIRS = [
	"rivetkit-typescript/packages",
	"shared/typescript",
	"engine/sdks/typescript",
];

interface BrowserEntry {
	default?: string;
	browser: string;
	browserName?: string;
}

interface StaticEntry {
	static: string;
}

type EntryValue = string | BrowserEntry | StaticEntry;

interface RivetKitConfig {
	outDir?: string;
	typesDir?: string;
	preBuild?: string;
	postBuild?: string;
	browserOutDir?: string;
	rootDir?: string;
	extraIncludes?: string[];
	extraExcludes?: string[];
	[key: string]: EntryValue | string | string[] | undefined;
}

function isEntry(key: string): boolean {
	return key.startsWith(".");
}

function isStaticEntry(value: EntryValue): value is StaticEntry {
	return typeof value === "object" && "static" in value;
}

function isBrowserEntry(value: EntryValue): value is BrowserEntry {
	return typeof value === "object" && "browser" in value && !("static" in value);
}

function getDefaultSource(value: EntryValue): string | undefined {
	if (typeof value === "string") return value;
	if (isBrowserEntry(value)) return value.default;
	return undefined;
}

function getBrowserSource(value: EntryValue): string | undefined {
	if (isBrowserEntry(value)) return value.browser;
	return undefined;
}

function stripSrcPrefix(source: string): string {
	return source.replace(/^src\//, "");
}

function changeExt(filePath: string, ext: string): string {
	return filePath.replace(/\.tsx?$/, ext);
}

function generateExports(
	config: RivetKitConfig,
	hasPathAliases: boolean,
): Record<string, unknown> {
	const outDir = config.outDir ?? "dist";
	const typesDir = config.typesDir ?? "dist/types";
	const browserOutDir = config.browserOutDir ?? "dist/browser";

	const exports: Record<string, unknown> = {};

	for (const [exportPath, value] of Object.entries(config)) {
		if (!isEntry(exportPath) || value === undefined) continue;
		const entry = value as EntryValue;

		if (isStaticEntry(entry)) {
			exports[exportPath] = `./${entry.static}`;
			continue;
		}

		const defaultSrc = getDefaultSource(entry);
		const browserSrc = getBrowserSource(entry);

		const jsPath = (src: string) =>
			`./${outDir}/${changeExt(stripSrcPrefix(src), ".js")}`;
		const cjsPath = (src: string) =>
			`./${outDir}/${changeExt(stripSrcPrefix(src), ".cjs")}`;
		const dtsPath = (src: string) =>
			`./${typesDir}/${changeExt(stripSrcPrefix(src), ".d.ts")}`;
		// Browser builds use named entries, falling back to source-derived path
		const browserName = isBrowserEntry(entry) ? (entry as BrowserEntry).browserName : undefined;
		const browserJsPath = (src: string) =>
			browserName
				? `./${browserOutDir}/${browserName}.js`
				: `./${browserOutDir}/${changeExt(stripSrcPrefix(src), ".js")}`;

		if (defaultSrc && browserSrc) {
			exports[exportPath] = {
				import: {
					browser: {
						types: dtsPath(browserSrc),
						default: browserJsPath(browserSrc),
					},
					types: dtsPath(defaultSrc),
					default: jsPath(defaultSrc),
				},
				require: {
					types: dtsPath(defaultSrc),
					default: cjsPath(defaultSrc),
				},
			};
		} else if (browserSrc && !defaultSrc) {
			exports[exportPath] = {
				import: {
					types: dtsPath(browserSrc),
					default: browserJsPath(browserSrc),
				},
			};
		} else if (defaultSrc) {
			exports[exportPath] = {
				import: {
					types: dtsPath(defaultSrc),
					default: jsPath(defaultSrc),
				},
				require: {
					types: dtsPath(defaultSrc),
					default: cjsPath(defaultSrc),
				},
			};
		}
	}

	if (hasPathAliases) {
		exports["./_internal/*"] = {
			types: `./${typesDir}/*.d.ts`,
		};
	}

	return exports;
}

function generateBuildScript(
	config: RivetKitConfig,
	hasPathAliases: boolean,
): string {
	const parts: string[] = [];
	const outDir = config.outDir ?? "dist";
	const browserOutDir = config.browserOutDir ?? "dist/browser";
	const browserInMainBuild = browserOutDir === outDir;

	// preBuild
	if (config.preBuild) {
		parts.push(config.preBuild);
	}

	// Collect tsup entry files
	const tsupEntries: string[] = [];
	for (const [key, value] of Object.entries(config)) {
		if (!isEntry(key) || value === undefined) continue;
		const entry = value as EntryValue;
		if (isStaticEntry(entry)) continue;
		const src = getDefaultSource(entry);
		if (src) tsupEntries.push(src);
		// Include browser sources in main build when they share the output dir
		if (browserInMainBuild) {
			const browserSrc = getBrowserSource(entry);
			if (browserSrc) tsupEntries.push(browserSrc);
		}
	}

	if (tsupEntries.length > 0) {
		parts.push(`tsup ${tsupEntries.join(" ")}`);
	}

	// tsc for declarations
	parts.push("tsc -p tsconfig.build.json --noCheck");

	// tsc-alias only needed when the package uses path aliases
	if (hasPathAliases) {
		parts.push("tsc-alias -p tsconfig.build.json");
	}

	// postBuild
	if (config.postBuild) {
		parts.push(config.postBuild);
	}

	return parts.join(" && ");
}

function generateBrowserBuildScript(config: RivetKitConfig): string | null {
	const outDir = config.outDir ?? "dist";
	const browserOutDir = config.browserOutDir ?? "dist/browser";

	// No separate browser build when browser entries share the main output dir
	if (browserOutDir === outDir) return null;

	// Check if there are any browser entries
	const hasBrowser = Object.entries(config).some(
		([key, value]) =>
			isEntry(key) && value !== undefined && isBrowserEntry(value as EntryValue),
	);

	if (!hasBrowser) return null;

	return "tsup --config tsup.browser.config.ts";
}

function checkPathAliases(pkgDir: string): boolean {
	const tsconfigPath = path.join(pkgDir, "tsconfig.json");
	if (!fs.existsSync(tsconfigPath)) return false;

	try {
		const content = fs.readFileSync(tsconfigPath, "utf8");
		if (!content.includes('"@/*"')) return false;
	} catch {
		return false;
	}

	// Verify source files actually use @/ imports so we don't add tsc-alias
	// unnecessarily to packages that declare the alias but never use it.
	const srcDir = path.join(pkgDir, "src");
	if (!fs.existsSync(srcDir)) return false;

	try {
		return hasAliaImportsRecursive(srcDir);
	} catch {
		return false;
	}
}

function hasAliaImportsRecursive(dir: string): boolean {
	for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
		if (entry.isDirectory()) {
			if (hasAliaImportsRecursive(path.join(dir, entry.name))) return true;
		} else if (entry.name.endsWith(".ts") || entry.name.endsWith(".tsx")) {
			const content = fs.readFileSync(path.join(dir, entry.name), "utf8");
			if (content.includes('from "@/') || content.includes('import "@/')) {
				return true;
			}
		}
	}
	return false;
}

function computeRelativeTsConfigBase(pkgDir: string): string {
	const basePath = path.join(ROOT, "tsconfig.base.json");
	return path.relative(pkgDir, basePath);
}

function generateTsConfigBuild(
	pkgDir: string,
	config: RivetKitConfig,
	hasPathAliases: boolean,
	pkg: Record<string, unknown>,
	references: Array<{ path: string }>,
): string {
	const relativePath = computeRelativeTsConfigBase(pkgDir);
	const rootDir = config.rootDir ?? "src";

	// Only include types: ["node"] if @types/node is available.
	// Otherwise override with empty array to prevent inheriting from base config.
	const deps = {
		...(pkg.dependencies as Record<string, string> ?? {}),
		...(pkg.devDependencies as Record<string, string> ?? {}),
	};
	const hasNodeTypes = "@types/node" in deps;

	const compilerOptions: Record<string, unknown> = {
		composite: true,
		types: hasNodeTypes ? ["node"] : [],
		resolveJsonModule: true,
		declaration: true,
		emitDeclarationOnly: true,
		declarationMap: true,
		outDir: "dist/types",
		declarationDir: "dist/types",
		rootDir,
		skipLibCheck: true,
	};

	if (hasPathAliases) {
		compilerOptions.paths = { "@/*": ["./src/*"] };
	}

	const includes = ["src/**/*", ...(config.extraIncludes ?? [])];
	const excludes = ["src/**/*.test.ts", ...(config.extraExcludes ?? [])];

	const tsconfig: Record<string, unknown> = {
		extends: relativePath,
		compilerOptions,
		include: includes,
		exclude: excludes,
	};

	if (references.length > 0) {
		tsconfig.references = references;
	}

	return JSON.stringify(tsconfig, null, "\t") + "\n";
}

function findPackages(): string[] {
	const packages: string[] = [];

	for (const dir of PACKAGE_DIRS) {
		const fullDir = path.join(ROOT, dir);
		if (!fs.existsSync(fullDir)) continue;

		for (const entry of fs.readdirSync(fullDir)) {
			const pkgJsonPath = path.join(fullDir, entry, "package.json");
			if (fs.existsSync(pkgJsonPath)) {
				packages.push(pkgJsonPath);
			}
		}
	}

	return packages;
}

function main() {
	const check = process.argv.includes("--check");
	let hasChanges = false;

	// Build a map of package name → directory for all managed packages.
	// Used to resolve workspace dependency references.
	const managedPackages = new Map<string, string>();
	for (const pkgJsonPath of findPackages()) {
		const raw = fs.readFileSync(pkgJsonPath, "utf8");
		const pkg = JSON.parse(raw);
		if (pkg.rivetkit && pkg.name) {
			managedPackages.set(pkg.name, path.dirname(pkgJsonPath));
		}
	}

	for (const pkgJsonPath of findPackages()) {
		const pkgDir = path.dirname(pkgJsonPath);
		const raw = fs.readFileSync(pkgJsonPath, "utf8");
		const pkg = JSON.parse(raw);

		const config: RivetKitConfig | undefined = pkg.rivetkit;
		if (!config) continue;

		const relPath = path.relative(ROOT, pkgJsonPath);
		const hasAliases = checkPathAliases(pkgDir);

		// Generate exports
		pkg.exports = generateExports(config, hasAliases);

		// Generate build script
		pkg.scripts = pkg.scripts || {};
		pkg.scripts.build = generateBuildScript(config, hasAliases);
		pkg.scripts["check-types"] = "tsc --noEmit";

		// Browser build script
		const browserBuild = generateBrowserBuildScript(config);
		if (browserBuild) {
			pkg.scripts["build:browser"] = browserBuild;
		}

		// Standard fields
		pkg.type = "module";
		if (!pkg.engines) pkg.engines = {};
		pkg.engines.node = ">=22.0.0";
		pkg.sideEffects = [
			"./dist/chunk-*.js",
			"./dist/chunk-*.cjs",
			"./dist/tsup/chunk-*.js",
			"./dist/tsup/chunk-*.cjs",
		];
		if (!pkg.files) pkg.files = [];
		if (!pkg.files.includes("dist")) pkg.files.push("dist");
		if (!pkg.files.includes("src")) pkg.files.push("src");
		if (!pkg.files.includes("package.json")) pkg.files.push("package.json");

		const newRaw = JSON.stringify(pkg, null, "\t") + "\n";

		if (newRaw !== raw) {
			hasChanges = true;
			if (check) {
				console.error(`  ${relPath}: out of sync`);
			} else {
				fs.writeFileSync(pkgJsonPath, newRaw);
				console.log(`  ${relPath}: updated`);
			}
		}

		// Compute project references from workspace dependencies
		const allDeps = {
			...(pkg.dependencies as Record<string, string> ?? {}),
			...(pkg.devDependencies as Record<string, string> ?? {}),
		};
		const references: Array<{ path: string }> = [];
		for (const depName of Object.keys(allDeps)) {
			const depDir = managedPackages.get(depName);
			if (!depDir || depDir === pkgDir) continue;
			const relRefPath = path.relative(pkgDir, depDir);
			references.push({ path: `${relRefPath}/tsconfig.build.json` });
		}

		// Generate or update tsconfig.build.json
		const tsconfigBuildPath = path.join(pkgDir, "tsconfig.build.json");
		const tsconfigContent = generateTsConfigBuild(pkgDir, config, hasAliases, pkg, references);
		const existingTsconfig = fs.existsSync(tsconfigBuildPath)
			? fs.readFileSync(tsconfigBuildPath, "utf8")
			: null;

		if (existingTsconfig !== tsconfigContent) {
			hasChanges = true;
			const action = existingTsconfig === null ? "created" : "updated";
			if (check) {
				console.error(`  ${path.relative(ROOT, tsconfigBuildPath)}: out of sync`);
			} else {
				fs.writeFileSync(tsconfigBuildPath, tsconfigContent);
				console.log(
					`  ${path.relative(ROOT, tsconfigBuildPath)}: ${action}`,
				);
			}
		}
	}

	if (check && hasChanges) {
		console.error(
			"\nRun 'tsx scripts/sync-package-json.ts' to fix.",
		);
		process.exit(1);
	}

	if (!check && !hasChanges) {
		console.log("All packages in sync.");
	}
}

main();
