/**
 * Node.js dependency injection module.
 *
 * This module provides a way to inject Node.js dependencies at runtime,
 * allowing the core rivetkit code to work in both Node.js and browser/edge
 * environments without conditional imports.
 *
 * In Node.js: Import from 'rivetkit' which uses the node-entry.ts entrypoint
 * that automatically injects all dependencies.
 *
 * In browser/edge: Import from 'rivetkit' which uses mod.ts directly.
 * Node-specific features will throw helpful errors if used.
 */

// Module-level state (set by node entrypoint)
let nodeDeps: NodeDependencies | null = null;

/**
 * Interface for all Node.js dependencies that need to be injected.
 */
export interface NodeDependencies {
	// Node.js built-ins
	fs: typeof import("node:fs");
	fsPromises: typeof import("node:fs/promises");
	path: typeof import("node:path");
	os: typeof import("node:os");
	childProcess: typeof import("node:child_process");
	crypto: typeof import("node:crypto");
	stream: typeof import("node:stream/promises");

	// Node-only npm packages
	getPort: typeof import("get-port").default;
	honoNodeServer: typeof import("@hono/node-server");
	honoNodeWs: typeof import("@hono/node-ws");
}

/**
 * Sets all Node.js dependencies. Called by the Node.js entrypoint.
 */
export function setNodeDependencies(deps: NodeDependencies): void {
	nodeDeps = deps;
}

/**
 * Checks if Node.js dependencies have been set.
 */
export function hasNodeDependencies(): boolean {
	return nodeDeps !== null;
}

/**
 * Gets all Node.js dependencies, throwing a helpful error if not set.
 */
function requireNodeDeps(): NodeDependencies {
	if (!nodeDeps) {
		throw new Error(
			"Node.js dependencies not available. " +
				"This feature requires Node.js. If you're in Node.js, ensure you're " +
				"importing from 'rivetkit' (not 'rivetkit/browser').",
		);
	}
	return nodeDeps;
}

// ============================================================================
// Node.js built-in module getters
// ============================================================================

/**
 * Gets the Node.js fs module.
 * @throws Error if Node.js dependencies are not set
 */
export function getNodeFsSync(): typeof import("node:fs") {
	return requireNodeDeps().fs;
}

/**
 * Gets the Node.js fs/promises module.
 * @throws Error if Node.js dependencies are not set
 */
export function getNodeFs(): typeof import("node:fs/promises") {
	return requireNodeDeps().fsPromises;
}

/**
 * Gets the Node.js path module.
 * @throws Error if Node.js dependencies are not set
 */
export function getNodePath(): typeof import("node:path") {
	return requireNodeDeps().path;
}

/**
 * Gets the Node.js os module.
 * @throws Error if Node.js dependencies are not set
 */
export function getNodeOs(): typeof import("node:os") {
	return requireNodeDeps().os;
}

/**
 * Gets the Node.js child_process module.
 * @throws Error if Node.js dependencies are not set
 */
export function getNodeChildProcess(): typeof import("node:child_process") {
	return requireNodeDeps().childProcess;
}

/**
 * Gets the Node.js crypto module.
 * @throws Error if Node.js dependencies are not set
 */
export function getNodeCrypto(): typeof import("node:crypto") {
	return requireNodeDeps().crypto;
}

/**
 * Gets the Node.js stream/promises module.
 * @throws Error if Node.js dependencies are not set
 */
export function getNodeStream(): typeof import("node:stream/promises") {
	return requireNodeDeps().stream;
}

// ============================================================================
// Node-only npm package getters
// ============================================================================

/**
 * Gets the get-port npm package.
 * @throws Error if Node.js dependencies are not set
 */
export function getGetPort(): typeof import("get-port").default {
	return requireNodeDeps().getPort;
}

/**
 * Gets the @hono/node-server npm package.
 * @throws Error if Node.js dependencies are not set
 */
export function getHonoNodeServer(): typeof import("@hono/node-server") {
	return requireNodeDeps().honoNodeServer;
}

/**
 * Gets the @hono/node-ws npm package.
 * @throws Error if Node.js dependencies are not set
 */
export function getHonoNodeWs(): typeof import("@hono/node-ws") {
	return requireNodeDeps().honoNodeWs;
}
