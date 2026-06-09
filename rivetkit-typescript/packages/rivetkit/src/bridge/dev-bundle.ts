import { createHash } from "node:crypto";
import { mkdirSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

/**
 * Development-mode child bundling.
 *
 * When the bridge host runs from TypeScript source (vitest, tsx), the worker
 * cannot execute the .ts child entry directly: Node has no resolver for the
 * package's path aliases inside worker threads, and loader hooks (tsx) break
 * CommonJS filename bookkeeping there. Instead the host prebundles a child
 * entry per definition with esbuild: package source (aliases included)
 * bundles into one ESM file, while every bare import stays external and
 * resolves from node_modules at runtime exactly like the production build.
 *
 * Built packages never hit this path; their child entry ships in dist.
 */

interface EsbuildModule {
	build(options: Record<string, unknown>): Promise<unknown>;
}

let esbuildModulePromise: Promise<EsbuildModule> | undefined;

// esbuild is not a direct dependency; resolve it through tsup, which pins the
// version used for package builds.
async function loadEsbuild(): Promise<EsbuildModule> {
	if (!esbuildModulePromise) {
		esbuildModulePromise = (async () => {
			const selfRequire = createRequire(import.meta.url);
			const tsupEntry = selfRequire.resolve("tsup");
			const tsupRequire = createRequire(tsupEntry);
			const esbuildEntry = tsupRequire.resolve("esbuild");
			const esbuildModule = (await import(
				pathToFileURL(esbuildEntry).href
			)) as EsbuildModule & { default?: EsbuildModule };
			const esbuild =
				typeof esbuildModule.build === "function"
					? esbuildModule
					: esbuildModule.default;
			if (!esbuild || typeof esbuild.build !== "function") {
				throw new Error("failed to load esbuild for bridge bundling");
			}
			return esbuild;
		})();
	}
	return esbuildModulePromise;
}

const PACKAGE_ROOT = fileURLToPath(new URL("../..", import.meta.url));
const CHILD_MAIN_PATH = fileURLToPath(
	new URL("./child-main.ts", import.meta.url),
);

function bundleRoot(): string {
	return path.join(process.cwd(), ".rivetkit", "bridge-dev-bundles");
}

const bundlePromises = new Map<string, Promise<string>>();

export interface DevBundleRequest {
	/** Stable cache key; identical requests share one bundle. */
	cacheKey: string;
	/** Generated entry source; import specifiers must be absolute paths. */
	entrySource: string;
}

/** Build (or reuse) a dev child bundle and return the bundled entry path. */
export async function ensureDevChildBundle(
	request: DevBundleRequest,
): Promise<string> {
	let promise = bundlePromises.get(request.cacheKey);
	if (!promise) {
		promise = buildBundle(request);
		bundlePromises.set(request.cacheKey, promise);
		promise.catch(() => {
			bundlePromises.delete(request.cacheKey);
		});
	}
	return promise;
}

async function buildBundle(request: DevBundleRequest): Promise<string> {
	const esbuild = await loadEsbuild();
	const hash = createHash("sha256")
		.update(request.cacheKey)
		.digest("hex")
		.slice(0, 16);
	const dir = path.join(bundleRoot(), hash);
	mkdirSync(dir, { recursive: true });
	const entryPath = path.join(dir, "entry.mjs");
	const outPath = path.join(dir, "child.mjs");
	writeFileSync(entryPath, request.entrySource);

	await esbuild.build({
		entryPoints: [entryPath],
		outfile: outPath,
		bundle: true,
		platform: "node",
		format: "esm",
		target: "node20",
		sourcemap: "inline",
		tsconfig: path.join(PACKAGE_ROOT, "tsconfig.json"),
		logLevel: "silent",
		plugins: [
			{
				name: "rivetkit-bridge-externals",
				setup(build: {
					onResolve: (
						options: { filter: RegExp },
						callback: (args: {
							path: string;
						}) => { path: string; external: boolean } | undefined,
					) => void;
				}) {
					// Bare specifiers resolve from node_modules at runtime;
					// only the package's own alias roots bundle from source.
					build.onResolve({ filter: /^[^./]/ }, (args) => {
						if (
							args.path.startsWith("@/") ||
							args.path === "rivetkit" ||
							args.path.startsWith("rivetkit/")
						) {
							return undefined;
						}
						return { path: args.path, external: true };
					});
				},
			},
		],
	});

	return outPath;
}

/** Generated entry for a worker-runtime actor definition module. */
export function moduleEntrySource(modulePath: string): string {
	return [
		`import * as moduleExports from ${JSON.stringify(modulePath)};`,
		`import { bootstrapBridgeChild } from ${JSON.stringify(CHILD_MAIN_PATH)};`,
		"void bootstrapBridgeChild({ moduleExports });",
		"",
	].join("\n");
}

/** Generated entry for a dynamic actor source file. */
export function sourceEntrySource(sourcePath: string): string {
	return [
		`import sourceModule from ${JSON.stringify(sourcePath)};`,
		`import { bootstrapBridgeChild } from ${JSON.stringify(CHILD_MAIN_PATH)};`,
		"void bootstrapBridgeChild({ sourceDefault: sourceModule });",
		"",
	].join("\n");
}
