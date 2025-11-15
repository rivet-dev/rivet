import { createRequire } from "node:module";

// Global variables for Node.js modules.
//
// We use synchronous require() instead of async import() for Node.js module loading because:
// 1. These modules are only needed in Node.js environments (not browser/edge)
// 2. registry.start() cannot be async and needs immediate access to Node modules
// 3. The setup process must be synchronous to avoid breaking the API
//
// Biome only allows imports of node modules in this file in order to ensure
// we're forcing the use of dynamic imports.
let nodeCrypto: typeof import("node:crypto") | undefined;
let nodeFsSync: typeof import("node:fs") | undefined;
let nodeFs: typeof import("node:fs/promises") | undefined;
let nodePath: typeof import("node:path") | undefined;
let nodeOs: typeof import("node:os") | undefined;
let nodeChildProcess: typeof import("node:child_process") | undefined;
let nodeStream: typeof import("node:stream/promises") | undefined;

let hasImportedDependencies = false;

// Helper to get a require function that works in both CommonJS and ESM.
// We use require() instead of await import() because registry.start() cannot
// be async and needs immediate access to Node.js modules during setup.
function getRequireFn() {
	// CommonJS context - use global require
	if (typeof require !== "undefined") {
		return require;
	}

	// ESM context - use createRequire with import.meta.url
	// @ts-expect-error - import.meta.url is available in ESM
	return createRequire(import.meta.url);
}

/**
 * Dynamically imports all required Node.js dependencies. We do this early in a
 * single function call in order to surface errors early.
 *
 * This function is idempotent and will only import once.
 *
 * @throws Error if Node.js modules are not available (e.g., in browser/edge environments)
 */
export function importNodeDependencies(): void {
	// Check if already loaded
	if (hasImportedDependencies) return;

	try {
		// Get a require function that works in both CommonJS and ESM
		const requireFn = getRequireFn();

		// Use requireFn with webpack ignore comment to prevent bundling
		// @ts-expect-error - dynamic require usage
		nodeCrypto = requireFn(/* webpackIgnore: true */ "node:crypto");
		// @ts-expect-error
		nodeFsSync = requireFn(/* webpackIgnore: true */ "node:fs");
		// @ts-expect-error
		nodeFs = requireFn(/* webpackIgnore: true */ "node:fs/promises");
		// @ts-expect-error
		nodePath = requireFn(/* webpackIgnore: true */ "node:path");
		// @ts-expect-error
		nodeOs = requireFn(/* webpackIgnore: true */ "node:os");
		// @ts-expect-error
		nodeChildProcess = requireFn(
			/* webpackIgnore: true */ "node:child_process",
		);
		// @ts-expect-error
		nodeStream = requireFn(
			/* webpackIgnore: true */ "node:stream/promises",
		);

		hasImportedDependencies = true;
	} catch (err) {
		console.warn(
			"Node.js modules not available, file system driver will not work",
			err,
		);
		throw err;
	}
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
