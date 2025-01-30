import { resolve } from "node:path";
import * as esbuild from "esbuild";
import { Input, Output } from "./mod.ts";
import { nodePolyfill } from "./node_polyfill.ts";
import { runPackageManagerCommand } from "./package_manager.ts";

export async function build(input: Input): Promise<Output> {
	console.log("Installing dependencies...");
	await runPackageManagerCommand(input.projectRoot, "install");

	let outfile = resolve(input.outDir, "index.js");
	console.log(`Building to output file: ${outfile}`);

	const result = await esbuild.build({
		entryPoints: [input.entryPoint],
		outfile,
		format: "esm",
		sourcemap: true,
		platform: "browser",
		// Helpful for traces & for RPCs when minified.
		keepNames: true,
		//plugins: [nodelessPlugin],
		plugins: [nodePolyfill(input)],
		external: [
			// Wasm must be loaded as a separate file manually, cannot be bundled
			"*.wasm",
			"*.wasm?module",
		],
		bundle: true,
		minify: input.bundle.minify,

		// Added new configurations
		target: ["esnext"],
		treeShaking: true,
		resolveExtensions: [".js", ".jsx", ".ts", ".tsx", ".json"],
		define: {
			"process.env.NODE_ENV": '"production"',
			// TODO: This is fixed in 2.0https://github.com/unjs/unenv/blob/8f5967b499f9b7f175f0d06938547f55430f316c/src/presets/nodeless.ts#L91
			"global": "globalThis",
		},

		// TODO: Remove any
		logLevel: input.bundle.logLevel as any,
		metafile: input.bundle.analyzeResult,
	});
	console.log("Build completed successfully");

	let analyzedMetafile: string | undefined = undefined;
	if (result.metafile) {
		console.log("Analyzing metafile...");
		analyzedMetafile = await esbuild.analyzeMetafile(result.metafile);
		console.log("Metafile analysis complete");
	}

	console.log("Build process finished");
	return {
		files: ["index.js"],
		analyzedMetafile,
	};
}
