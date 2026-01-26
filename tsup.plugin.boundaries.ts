/// <reference types="@types/node" />

import { builtinModules } from "node:module";
import { readFile } from "node:fs/promises";
import type { Plugin } from "esbuild";

/**
 * Creates an esbuild plugin that enforces browser/node code boundaries.
 *
 * This plugin:
 * - Prevents .browser files from importing .node files
 * - Prevents .node files from importing .browser files
 * - Prevents .browser files from importing Node.js built-in modules
 * - Auto-detects which external packages use Node.js built-ins in browser context
 *
 * File naming convention:
 * - *.browser.ts/tsx - Browser-only code (can use DOM, window, etc.)
 * - *.node.ts/tsx - Node-only code (can use fs, path, crypto, etc.)
 * - *.ts/tsx - Universal/shared code (no platform-specific APIs)
 *
 * Note: This uses onLoad instead of onResolve because tsup's `external` config
 * prevents onResolve from being called for Node built-in modules.
 */
export function createBoundaryEnforcementPlugin(): Plugin {
	const nodeBuiltinSet = new Set(builtinModules);

	// Add node: prefixed versions
	for (const mod of builtinModules) {
		nodeBuiltinSet.add(`node:${mod}`);
	}

	// Regex to extract import/require statements
	const importRegex =
		/(?:import\s+(?:[\w*{}\s,]+\s+from\s+)?|require\s*\(\s*)['"]([^'"]+)['"]/g;

	function isNodeBuiltin(modulePath: string): boolean {
		const moduleName = modulePath.replace("node:", "").split("/")[0];
		return nodeBuiltinSet.has(modulePath) || nodeBuiltinSet.has(moduleName);
	}

	function isNodeFile(importPath: string): boolean {
		return /\.node(\.(ts|tsx|js|jsx|mts|mjs|cts|cjs))?$/.test(importPath);
	}

	function isBrowserFile(importPath: string): boolean {
		return /\.browser(\.(ts|tsx|js|jsx|mts|mjs|cts|cjs))?$/.test(importPath);
	}

	return {
		name: "enforce-boundaries",
		setup(build) {
			// Check .browser files for violations
			build.onLoad(
				{ filter: /\.browser\.(ts|tsx|js|jsx|mts|mjs|cts|cjs)$/ },
				async (args) => {
					const contents = await readFile(args.path, "utf8");
					const errors: { text: string; location?: { file: string; line: number; column: number } }[] = [];

					// Find all imports
					let match: RegExpExecArray | null;
					while ((match = importRegex.exec(contents)) !== null) {
						const importPath = match[1];

						// Check for Node built-in imports
						if (isNodeBuiltin(importPath)) {
							// Find line number
							const lineNumber =
								contents.slice(0, match.index).split("\n").length;
							errors.push({
								text: `❌ BOUNDARY VIOLATION: Cannot use Node.js built-in "${importPath}" in browser context`,
								location: {
									file: args.path,
									line: lineNumber,
									column: 0,
								},
							});
						}

						// Check for .node file imports
						if (isNodeFile(importPath)) {
							const lineNumber =
								contents.slice(0, match.index).split("\n").length;
							errors.push({
								text: `❌ BOUNDARY VIOLATION: Cannot import .node module "${importPath}" in browser context`,
								location: {
									file: args.path,
									line: lineNumber,
									column: 0,
								},
							});
						}
					}

					// Reset regex state
					importRegex.lastIndex = 0;

					if (errors.length > 0) {
						return { errors };
					}

					// Return undefined to let esbuild handle the file normally
					return undefined;
				},
			);

			// Check .node files for .browser imports
			build.onLoad(
				{ filter: /\.node\.(ts|tsx|js|jsx|mts|mjs|cts|cjs)$/ },
				async (args) => {
					const contents = await readFile(args.path, "utf8");
					const errors: { text: string; location?: { file: string; line: number; column: number } }[] = [];

					// Find all imports
					let match: RegExpExecArray | null;
					while ((match = importRegex.exec(contents)) !== null) {
						const importPath = match[1];

						// Check for .browser file imports
						if (isBrowserFile(importPath)) {
							const lineNumber =
								contents.slice(0, match.index).split("\n").length;
							errors.push({
								text: `❌ BOUNDARY VIOLATION: Cannot import .browser module "${importPath}" in Node context`,
								location: {
									file: args.path,
									line: lineNumber,
									column: 0,
								},
							});
						}
					}

					// Reset regex state
					importRegex.lastIndex = 0;

					if (errors.length > 0) {
						return { errors };
					}

					// Return undefined to let esbuild handle the file normally
					return undefined;
				},
			);
		},
	};
}
