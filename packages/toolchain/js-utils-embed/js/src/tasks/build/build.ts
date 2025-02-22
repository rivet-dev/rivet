import { resolve } from "node:path";
import * as esbuild from "esbuild";
import { Input, Output } from "./mod.ts";
import { runPackageManagerCommand } from "./package_manager.ts";
import stdLibPlugin from "node-stdlib-browser/helpers/esbuild/plugin";
import stdLibBrowserFull from "node-stdlib-browser";
import { createRequire } from "node:module";
import pkgDir from "pkg-dir";

// HACK(RVT-4598): Copied from node-stdlib-browser to reproduce functionality
const resolvePath = (path: string) => {
	let resolvedPath: string;
	try {
		resolvedPath = require.resolve(path);
	} catch {
		resolvedPath = (
			globalThis.require ?? createRequire(import.meta.url)
		).resolve(path);
	}
	if (!path.includes('./')) {
		const directory = pkgDir.sync(resolvedPath) ?? '';
		return directory;
	}
	return resolvedPath;
};

export async function build(input: Input): Promise<Output> {
	console.log("Installing dependencies...");
	await runPackageManagerCommand(input.projectRoot, "install");

	let outfile = resolve(input.outDir, "index.js");
	console.log(`Building to output file: ${outfile}`);

	const stdLibInject = resolve(Deno.cwd(), "node_modules", "node-stdlib-browser", "helpers", "esbuild", "shim");

	// HACK(RVT-4598): Override stream imports with correct stream version
	const overrideLibs = {
		// We need to be using readable-stream 4.x, but node-stdlib-browser provides 3.x which doesn't support all browser functionality
		stream: resolvePath("readable-stream"),
		_stream_duplex: resolvePath('readable-stream/lib/_stream_duplex.js'),
		_stream_passthrough: resolvePath(
			'readable-stream/lib/_stream_passthrough.js'
		),
		_stream_readable :resolvePath('readable-stream/lib/_stream_readable.js'),
		_stream_transform : resolvePath(
			'readable-stream/lib/_stream_transform.js'
		),
		_stream_writable : resolvePath('readable-stream/lib/_stream_writable.js'),
	};

	let stdLibBrowser = stdLibBrowserFull;
	for (const [packageName, packagePath] of Object.entries(overrideLibs)) {
		stdLibBrowser[packageName] = packagePath;
		stdLibBrowser[ `node:${packageName}` ] = packagePath;
	}

	const result = await esbuild.build({
		absWorkingDir: input.projectRoot,
		entryPoints: [input.entryPoint],
		outfile,
		format: "esm",
		sourcemap: true,
		platform: "browser",
		// Helpful for traces & for RPCs when minified.
		keepNames: true,
		//plugins: [nodePolyfill(input)],
		inject: [stdLibInject],
		plugins: [stdLibPlugin(stdLibBrowser)],
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
			global: 'global',
			process: 'process',
			Buffer: 'Buffer',

			"process.env.NODE_ENV": '"production"',
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

