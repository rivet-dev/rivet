// Global variables for Node.js modules
let nodeCrypto: typeof import("node:crypto") | undefined;
let nodeFsSync: typeof import("node:fs") | undefined;
let nodeFs: typeof import("node:fs/promises") | undefined;
let nodePath: typeof import("node:path") | undefined;
let nodeOs: typeof import("node:os") | undefined;
let nodeChildProcess: typeof import("node:child_process") | undefined;
let nodeStream: typeof import("node:stream/promises") | undefined;

// Singleton promise to ensure imports happen only once
let importPromise: Promise<void> | undefined;

/**
 * Dynamically imports all required Node.js dependencies.
 * This function is idempotent and will only import once.
 * @throws Error if Node.js modules are not available (e.g., in browser/edge environments)
 */
export async function importNodeDependencies(): Promise<void> {
	if (importPromise) return importPromise;

	importPromise = (async () => {
		try {
			// Dynamic imports with webpack ignore comment to prevent bundling
			const cryptoModule = "node:crypto";
			const fsModule = "node:fs";
			const fsPromisesModule = "node:fs/promises";
			const pathModule = "node:path";
			const osModule = "node:os";
			const childProcessModule = "node:child_process";
			const streamModule = "node:stream/promises";

			const modules = await Promise.all([
				import(/* webpackIgnore: true */ cryptoModule),
				import(/* webpackIgnore: true */ fsModule),
				import(/* webpackIgnore: true */ fsPromisesModule),
				import(/* webpackIgnore: true */ pathModule),
				import(/* webpackIgnore: true */ osModule),
				import(/* webpackIgnore: true */ childProcessModule),
				import(/* webpackIgnore: true */ streamModule),
			]);

			[
				nodeCrypto,
				nodeFsSync,
				nodeFs,
				nodePath,
				nodeOs,
				nodeChildProcess,
				nodeStream,
			] = modules;
		} catch (err) {
			// Node.js not available - will use memory driver fallback
			console.warn(
				"Node.js modules not available, file system driver will not work",
				err,
			);
			throw err;
		}
	})();

	return importPromise;
}

/**
 * Gets the Node.js crypto module.
 * @throws Error if crypto module is not loaded
 */
export function getNodeCrypto(): typeof import("node:crypto") {
	if (!nodeCrypto) {
		throw new Error(
			"Node crypto module not loaded. Ensure importNodeDependencies() has been called.",
		);
	}
	return nodeCrypto;
}

/**
 * Gets the Node.js fs module.
 * @throws Error if fs module is not loaded
 */
export function getNodeFsSync(): typeof import("node:fs") {
	if (!nodeFsSync) {
		throw new Error(
			"Node fs module not loaded. Ensure importNodeDependencies() has been called.",
		);
	}
	return nodeFsSync;
}

/**
 * Gets the Node.js fs/promises module.
 * @throws Error if fs/promises module is not loaded
 */
export function getNodeFs(): typeof import("node:fs/promises") {
	if (!nodeFs) {
		throw new Error(
			"Node fs/promises module not loaded. Ensure importNodeDependencies() has been called.",
		);
	}
	return nodeFs;
}

/**
 * Gets the Node.js path module.
 * @throws Error if path module is not loaded
 */
export function getNodePath(): typeof import("node:path") {
	if (!nodePath) {
		throw new Error(
			"Node path module not loaded. Ensure importNodeDependencies() has been called.",
		);
	}
	return nodePath;
}

/**
 * Gets the Node.js os module.
 * @throws Error if os module is not loaded
 */
export function getNodeOs(): typeof import("node:os") {
	if (!nodeOs) {
		throw new Error(
			"Node os module not loaded. Ensure importNodeDependencies() has been called.",
		);
	}
	return nodeOs;
}

/**
 * Gets the Node.js child_process module.
 * @throws Error if child_process module is not loaded
 */
export function getNodeChildProcess(): typeof import("node:child_process") {
	if (!nodeChildProcess) {
		throw new Error(
			"Node child_process module not loaded. Ensure importNodeDependencies() has been called.",
		);
	}
	return nodeChildProcess;
}

/**
 * Gets the Node.js stream/promises module.
 * @throws Error if stream/promises module is not loaded
 */
export function getNodeStream(): typeof import("node:stream/promises") {
	if (!nodeStream) {
		throw new Error(
			"Node stream/promises module not loaded. Ensure importNodeDependencies() has been called.",
		);
	}
	return nodeStream;
}

/**
 * Checks if Node.js dependencies are available.
 * @returns true if all Node.js modules are loaded
 */
export function areNodeDependenciesAvailable(): boolean {
	return !!(
		nodeCrypto &&
		nodeFsSync &&
		nodeFs &&
		nodePath &&
		nodeOs &&
		nodeChildProcess &&
		nodeStream
	);
}
