import { resolve } from "node:path";
import dedent from "dedent";
import { env, nodeless } from "unenv";
import type { Plugin, PluginBuild } from "esbuild";
import { decodeGlobalName, encodeGlobalName } from "./utils";

const NODE_BUILTIN_MODULES_NAMESPACE = "node-built-in-modules";
const UNENV_ALIAS_NAMESPACE = "required-unenv-alias";

// Based on https://github.com/unjs/unenv/issues/32#issuecomment-1125928455

export function nodePolyfill(): Plugin {
	const { alias, inject, external } = env(nodeless);
	const nodeBuiltinModules = Object.keys(alias);

	return {
		name: "node-polyfill",
		setup(build) {
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

function handleUnenvModuleAliases(
	build: PluginBuild,
	moduleAliases: Record<string, string>,
	externalModules: string[]
) {
	const resolvedAliases: Record<string, string> = {};
	for (const [moduleName, aliasPath] of Object.entries(moduleAliases)) {
		try {
			resolvedAliases[moduleName] = require
				.resolve(aliasPath)
				.replace(/\.cjs$/, ".mjs");
		} catch (e) {
			// this is an alias for package that is not installed in the current app => ignore
		}
	}

	const ALIAS_MODULE_PATTERN = new RegExp(
		`^(${Object.keys(resolvedAliases).join("|")})$`
	);

	build.onResolve({ filter: ALIAS_MODULE_PATTERN }, (args) => {
		const aliasPath = moduleAliases[args.path];
		// Convert `require()` calls for NPM packages to a virtual ES Module that can be imported avoiding the require calls.
		// Note: Does not apply to Node.js packages that are handled in `handleRequireCallsToNodeJSBuiltins`
		if (
			args.kind === "require-call" &&
			(aliasPath.startsWith("unenv/runtime/npm/") ||
				aliasPath.startsWith("unenv/runtime/mock/"))
		) {
			return {
				path: args.path,
				namespace: UNENV_ALIAS_NAMESPACE,
			};
		}

		// Resolve the alias to its absolute path and potentially mark it as external
		return {
			path: resolvedAliases[args.path],
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
	globalPolyfills: Record<string, string | string[]>
) {
	const VIRTUAL_POLYFILL_PATTERN = /__unenv_polyfill-([^.]+)\.js$/;
	const virtualModulePrefix = resolve(
		process.cwd(),
		"__unenv_polyfill-"
	);

	build.initialOptions.inject = [
		...(build.initialOptions.inject ?? []),
		...Object.keys(globalPolyfills).map(
			(globalName) => `${virtualModulePrefix}${encodeGlobalName(globalName)}.js`
		)
	];

	build.onResolve({ filter: VIRTUAL_POLYFILL_PATTERN }, ({ path }) => ({ path }));

	build.onLoad({ filter: VIRTUAL_POLYFILL_PATTERN }, ({ path }) => {
		const globalName = decodeGlobalName(path.match(VIRTUAL_POLYFILL_PATTERN)![1]);
		const { importStatement, exportName } = createGlobalPolyfillImport(globalPolyfills[globalName]);

		return {
			contents: dedent`
				  ${importStatement}
				  globalThis.${globalName} = ${exportName};
			  `
		};
	});
}

function createGlobalPolyfillImport(polyfillConfig: string | string[]) {
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
