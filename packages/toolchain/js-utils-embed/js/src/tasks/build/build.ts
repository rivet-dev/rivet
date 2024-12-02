import { resolve } from "@std/path";
import { denoPlugins } from "@rivet-gg/esbuild-deno-loader";
import * as esbuild from "esbuild";
import { Input, Output } from "./mod.ts";

export async function build(input: Input): Promise<Output> {
	let outfile = resolve(input.outDir, "index.js");
	const result = await esbuild.build({
		entryPoints: [input.entryPoint],
		outfile,
		format: "esm",
		sourcemap: true,
		plugins: [
			// Bundle Deno dependencies
			...denoPlugins({
				loader: "native",
				configPath: input.deno.configPath,
				importMapURL: input.deno.importMapUrl,
				lockPath: input.deno.lockPath,
			}),

			// HACK: esbuild-deno-loader does not play nice with
			// Windows paths, so we manually resolve any paths that
			// start with a Windows path separator (\) and resolve
			// them to the full path.
			// {
			// 	name: "fix-windows-paths",
			// 	setup(build: esbuild.PluginBuild) {
			// 		build.onResolve({ filter: /^\\.*/ }, (args) => {
			// 			const resolvedPath = resolve(args.resolveDir, args.path);
			// 			if (!exists(resolvedPath, { isFile: true })) {
			// 				return {
			// 					errors: [{ text: `File could not be resolved: ${resolvedPath}` }],
			// 				};
			// 			}

			// 			return {
			// 				path: resolve(args.resolveDir, args.path),
			// 			};
			// 		});
			// 	},
			// } satisfies esbuild.Plugin,
		],
		define: {
			// HACK: Disable `process.domain` in order to correctly handle this edge case:
			// https://github.com/brianc/node-postgres/blob/50c06f9bc6ff2ca1e8d7b7268b9af54ce49d72c1/packages/pg/lib/native/query.js#L126
			"process.domain": "undefined",
		},
		external: [
			// Provided by Deno
			"node:*",

			// Wasm must be loaded as a separate file manually, cannot be bundled
			"*.wasm",
			"*.wasm?module",
		],
		bundle: true,
		minify: input.bundle.minify,

		// TODO: Remove any
		logLevel: input.bundle.logLevel as any,
		metafile: input.bundle.analyzeResult,
	});

	let analyzedMetafile = undefined;
	if (result.metafile) {
		analyzedMetafile = await esbuild.analyzeMetafile(result.metafile);
	}

	return {
		files: ["index.js"],
		analyzedMetafile,
	};
}
