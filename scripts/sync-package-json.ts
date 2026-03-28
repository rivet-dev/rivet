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
	[key: string]: EntryValue | string | undefined;
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

function generateBuildScript(config: RivetKitConfig): string {
	const parts: string[] = [];

	// preBuild
	if (config.preBuild) {
		parts.push(config.preBuild);
	}

	// Collect tsup entry files (default sources only, not browser-only)
	const tsupEntries: string[] = [];
	for (const [key, value] of Object.entries(config)) {
		if (!isEntry(key) || value === undefined) continue;
		const entry = value as EntryValue;
		if (isStaticEntry(entry)) continue;
		const src = getDefaultSource(entry);
		if (src) tsupEntries.push(src);
	}

	if (tsupEntries.length > 0) {
		parts.push(`tsup ${tsupEntries.join(" ")}`);
	}

	// tsc for declarations + tsc-alias for path rewriting
	parts.push("tsc -p tsconfig.build.json --noCheck");
	parts.push("tsc-alias -p tsconfig.build.json");

	// postBuild
	if (config.postBuild) {
		parts.push(config.postBuild);
	}

	return parts.join(" && ");
}

function generateBrowserBuildScript(config: RivetKitConfig): string | null {
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
		// Simple check: look for "@/*" in the paths config
		return content.includes('"@/*"');
	} catch {
		return false;
	}
}

function computeRelativeTsConfigBase(pkgDir: string): string {
	const basePath = path.join(ROOT, "tsconfig.base.json");
	return path.relative(pkgDir, basePath);
}

function generateTsConfigBuild(pkgDir: string): string {
	const relativePath = computeRelativeTsConfigBase(pkgDir);

	const tsconfig: Record<string, unknown> = {
		extends: relativePath,
		compilerOptions: {
			types: ["node"],
			resolveJsonModule: true,
			declaration: true,
			emitDeclarationOnly: true,
			outDir: "dist/types",
			declarationDir: "dist/types",
			rootDir: "src",
			skipLibCheck: true,
		},
		include: ["src/**/*"],
		exclude: ["src/**/*.test.ts"],
	};

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
		pkg.scripts.build = generateBuildScript(config);
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

		// Generate tsconfig.build.json if it doesn't exist
		const tsconfigBuildPath = path.join(pkgDir, "tsconfig.build.json");
		if (!fs.existsSync(tsconfigBuildPath)) {
			const tsconfigContent = generateTsConfigBuild(pkgDir);
			if (check) {
				console.error(`  ${path.relative(ROOT, tsconfigBuildPath)}: missing`);
				hasChanges = true;
			} else {
				fs.writeFileSync(tsconfigBuildPath, tsconfigContent);
				console.log(
					`  ${path.relative(ROOT, tsconfigBuildPath)}: created`,
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
