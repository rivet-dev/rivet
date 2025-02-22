import { resolve } from "node:path";
import dedent from "dedent";
import { defineEnv, type Preset } from "unenv";
import type { Plugin, PluginBuild } from "esbuild";
import { decodeGlobalName, encodeGlobalName } from "./utils.ts";
import { Input } from "./mod.ts";
import { rivetPreset } from "./preset.ts";
import { existsSync } from "@std/fs";

const NODE_BUILTIN_MODULES_NAMESPACE = "node-built-in-modules";
const UNENV_ALIAS_NAMESPACE = "required-unenv-alias";

// Based on https://github.com/unjs/unenv/issues/32#issuecomment-1125928455

export function nodePolyfill(_input: Input): Plugin {
	const { env: { alias, inject, external } } = defineEnv({
		presets: [rivetPreset]
	});
	const nodeBuiltinModules = Object.keys(alias);

	return {
		name: "node-polyfill",
		async setup(build) {
			handleNodeBuiltinModules(build, nodeBuiltinModules);
			handleUnenvModuleAliases(build, alias, external);
			handleGlobalPolyfills(build, inject);
		},
	};
}

function handleNodeBuiltinModules(build: PluginBuild, nodeBuiltinModules: string[]) {
	const NODE_BUILTIN_MODULES_PATTERN = new RegExp(`^(${nodeBuiltinModules.join("|")})$`);

	build.onResolve({ filter: NODE_BUILTIN_MODULES_PATTERN }, (args) => {
		if (args.kind === "require-call") {
			//console.log("Resolving node builtin require", args.path);
			return {
				path: args.path,
				namespace: NODE_BUILTIN_MODULES_NAMESPACE,
			};
		}
	});
	build.onLoad(
		{ filter: /.*/, namespace: NODE_BUILTIN_MODULES_NAMESPACE },
		({ path }) => {
			return {
				contents: dedent`
					import libDefault from '${path}';
					module.exports = libDefault;`,
				loader: "js",
			};
		}
	);
}

/**
 * Handles injecting unenv polyfills.
 * 
 * - moduleAliases = node import -> path in unenv
 * - externalModules = modules that should resolve to native imports (only relevant when targeting node, will be empty on web targets)
 */
function handleUnenvModuleAliases(
	build: PluginBuild,
	moduleAliases: Record<string, string>,
	externalModules: string[]
) {
	const resolvedAliases: Record<string, string> = {};
	for (const [moduleName, aliasPath] of Object.entries(moduleAliases)) {
		const modifiedPath = `${aliasPath.replace(/^unenv\//, "").replace(/-cjs$/, "")}.mjs`;
		const resolvedPath = resolve(Deno.cwd(), "node_modules", "unenv", "dist", "runtime", modifiedPath);
		if (existsSync(resolvedPath)) {
			//console.log(`Resolved ${moduleName} -> ${aliasPath} -> ${resolvedPath}`);

			// HACK: Unsure why it's giving us `-cjs` instead of `.cjs`
			resolvedAliases[moduleName] = resolvedPath;
		} else {
			// This will fail to resolve buffer.mjs, consola, and mime
			//console.log(`Failed to resolve ${moduleName} -> ${aliasPath} -> ${resolvedPath}`);
		}
	}
	//console.log('Resolved aliases', resolvedAliases);

	const ALIAS_MODULE_PATTERN = new RegExp(
		`^(${Object.keys(resolvedAliases).join("|")})$`
	);

	build.onResolve({ filter: ALIAS_MODULE_PATTERN }, (args) => {
		const aliasPath = moduleAliases[args.path];
		// Convert `require()` calls for NPM packages to a virtual ES Module that can be imported avoiding the require calls.
		// Note: Does not apply to Node.js packages that are handled in `handleRequireCallsToNodeJSBuiltins`
		if (
			(args.kind === "import-statement" || args.kind === "require-call") &&
			(aliasPath.startsWith("unenv/runtime/npm/") ||
				aliasPath.startsWith("unenv/runtime/mock/"))
		) {
			return {
				path: args.path,
				namespace: UNENV_ALIAS_NAMESPACE,
			};
		}

		const resolvedPath = resolvedAliases[args.path];
		//console.log(`Resolving unenv module alias ${args.path} -> ${resolvedPath}`);

		// Resolve the alias to its absolute path and potentially mark it as external
		return {
			path: resolvedPath,
			external: externalModules.includes(aliasPath),
		};
	});

	build.initialOptions.banner = { js: "", ...build.initialOptions.banner };
	build.initialOptions.banner.js += dedent`
		function convertEsmToCjs(esmModule) {
			const cjsModule = 'default' in esmModule ? esmModule.default : {};
			for (const [k, v] of Object.entries(esmModule)) {
				if (k !== 'default') {
					Object.defineProperty(cjsModule, k, {
						enumerable: true,
						value: v,
					});
				}
			}
			return cjsModule;
		}
		`;

	build.onLoad(
		{ filter: /.*/, namespace: UNENV_ALIAS_NAMESPACE },
		({ path }) => {
			return {
				contents: dedent`
					import * as esm from '${path}';
					module.exports = convertEsmToCjs(esm);
				`,
				loader: "js",
			};
		}
	);
}

function handleGlobalPolyfills(
	build: PluginBuild,
	inject: Record<string, string | string[]>
) {

	const VIRTUAL_POLYFILL_PATTERN = /__unenv_polyfill-([^.]+)\.js$/;
	const virtualModulePrefix = resolve(
		process.cwd(),
		"__unenv_polyfill-"
	);

	build.initialOptions.inject = [
		...(build.initialOptions.inject ?? []),
		...Object.keys(inject).map(
			(globalName) =>  {
				return `${virtualModulePrefix}${encodeGlobalName(globalName)}.js`
			}
		)
	];

	build.onResolve({ filter: VIRTUAL_POLYFILL_PATTERN }, ({ path }) => ({ path }));

	build.onLoad({ filter: VIRTUAL_POLYFILL_PATTERN }, ({ path }) => {
		const globalName = decodeGlobalName(path.match(VIRTUAL_POLYFILL_PATTERN)![1]);
		const { importStatement, exportName } = createGlobalPolyfillImport(inject[globalName]);

		return {
			contents: dedent`
				  ${importStatement}
				  globalThis.${globalName} = ${exportName};
			  `
		};
	});
}

function createGlobalPolyfillImport(polyfillConfig: string | string[]) {
	// Overwrite bugged configs
	// See https://github.com/unjs/unenv/commit/d9d4d035c7ef13fb03189b49ec95ee4b14d1a603 for v2
	if (Array.isArray(polyfillConfig) && polyfillConfig[0] === "buffer") {
		polyfillConfig = ["unenv/runtime/node/buffer/index", "Buffer"];
	} else if (typeof polyfillConfig === "string" && polyfillConfig == "unenv/runtime/node/process") {
		polyfillConfig = "unenv/runtime/node/process/index";
	}

	// Build import statement
	if (typeof polyfillConfig === "string") {
		return {
			importStatement: `import nodePolyfill from "${polyfillConfig}";`,
			exportName: "nodePolyfill"
		};
	}
	const [moduleSpecifier, exportName] = polyfillConfig;
	return {
		importStatement: `import { ${exportName} } from "${moduleSpecifier}";`,
		exportName
	};
}
