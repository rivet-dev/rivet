/**
 * Native SQLite integration via @rivetkit/sqlite-native.
 *
 * Attempts to load the native addon at runtime and provides a fallback-aware
 * API for the database provider. The KV channel connection is initialized once
 * per process and shared across all actors.
 *
 * The native VFS and WASM VFS are byte-compatible. See
 * rivetkit-typescript/packages/sqlite-native/src/vfs.rs and
 * rivetkit-typescript/packages/sqlite-vfs/src/vfs.ts.
 */

import { getRequireFn } from "@/utils/node";
import {
	getRivetEndpoint,
	getRivetToken,
	getRivetNamespace,
} from "@/utils/env-vars";
import type { RawAccess } from "./config";
import { AsyncMutex } from "./shared";

// Type declarations for @rivetkit/sqlite-native.
// Declared inline to avoid a build-time dependency on the native addon,
// which may not be installed or compiled.
interface NativeSqliteModule {
	connect(config: {
		url: string;
		token?: string;
		namespace: string;
	}): NativeKvChannel;
	openDatabase(
		channel: NativeKvChannel,
		actorId: string,
	): NativeDatabase;
	execute(
		db: NativeDatabase,
		sql: string,
		params?: unknown[],
	): Promise<{ changes: number }>;
	query(
		db: NativeDatabase,
		sql: string,
		params?: unknown[],
	): Promise<{ columns: string[]; rows: unknown[][] }>;
	exec(
		db: NativeDatabase,
		sql: string,
	): Promise<{ columns: string[]; rows: unknown[][] }>;
	closeDatabase(db: NativeDatabase): void;
	disconnect(channel: NativeKvChannel): void;
}

// Opaque handles from the native addon.
type NativeKvChannel = object;
type NativeDatabase = object;

// Cached detection result.
let nativeModule: NativeSqliteModule | null = null;
let detectionDone = false;

// Singleton KV channel connection, shared across all actors in this process.
let kvChannel: NativeKvChannel | null = null;

// Tracks whether the singleton channel was explicitly disconnected. When true,
// getOrCreateKvChannel() will create a fresh channel instead of reusing the dead
// handle. This covers both process shutdown cleanup and manual disconnect().
let channelDisconnected = false;

// Whether the process shutdown handler has been registered.
let shutdownRegistered = false;

/**
 * Reset the cached native SQLite detection state.
 * For testing only. Allows tests to switch between native and WASM VFS
 * backends within the same process.
 *
 * @param disable - If true, force detection to report native as unavailable.
 *                  If false/undefined, reset so the next call re-detects.
 * @internal
 */
export function _resetNativeDetection(disable?: boolean): void {
	if (kvChannel && nativeModule) {
		try {
			nativeModule.disconnect(kvChannel);
		} catch {
			// Ignore cleanup errors
		}
	}
	kvChannel = null;
	channelDisconnected = false;

	if (disable) {
		detectionDone = true;
		nativeModule = null;
	} else {
		detectionDone = false;
		nativeModule = null;
	}
}

/**
 * Attempts to load the @rivetkit/sqlite-native .node addon.
 * Catches all failure modes: missing file, glibc mismatch,
 * N-API version mismatch, corrupted binary.
 */
export function nativeSqliteAvailable(): boolean {
	if (detectionDone) return nativeModule !== null;
	detectionDone = true;

	try {
		const requireFn = getRequireFn();
		nativeModule = requireFn(
			/* webpackIgnore: true */ "@rivetkit/sqlite-native",
		) as NativeSqliteModule;
		return true;
	} catch {
		nativeModule = null;
		return false;
	}
}

/**
 * Returns the loaded native module. Only valid after nativeSqliteAvailable()
 * returns true.
 */
function getNativeModule(): NativeSqliteModule {
	if (!nativeModule) {
		throw new Error("native SQLite module not loaded");
	}
	return nativeModule;
}

/**
 * Disconnect the singleton KV channel if it exists. Safe to call multiple times.
 */
export function disconnectKvChannel(): void {
	if (kvChannel && nativeModule) {
		try {
			nativeModule.disconnect(kvChannel);
		} catch {
			// Ignore cleanup errors during shutdown.
		}
	}
	kvChannel = null;
	channelDisconnected = true;
}

/**
 * Register process shutdown handlers that clean up the singleton KV channel.
 * Called once per process on first channel creation. Uses `beforeExit` for
 * graceful exit and signal handlers for SIGTERM/SIGINT.
 */
function registerShutdownHandler(): void {
	if (shutdownRegistered) return;
	shutdownRegistered = true;

	const onShutdown = () => {
		disconnectKvChannel();
	};

	// beforeExit fires when the event loop drains. Signals fire on external
	// termination. Both paths call disconnectKvChannel which is idempotent.
	process.on("beforeExit", onShutdown);
	process.on("SIGTERM", onShutdown);
	process.on("SIGINT", onShutdown);
}

/**
 * Get or create the process-level KV channel connection.
 *
 * Derives the WebSocket URL from RIVET_ENDPOINT (defaults to
 * http://127.0.0.1:6420 for local dev). Authenticates with RIVET_TOKEN.
 *
 * If the channel was previously disconnected (e.g., during shutdown or due
 * to a permanent failure), a new channel is created automatically.
 */
function getOrCreateKvChannel(): NativeKvChannel {
	// Recreate the channel if it was explicitly disconnected.
	if (kvChannel && !channelDisconnected) return kvChannel;

	const mod = getNativeModule();
	const endpoint = getRivetEndpoint() ?? "http://127.0.0.1:6420";
	const token = getRivetToken();
	const namespace = getRivetNamespace() ?? "default";

	// Convert HTTP(S) endpoint to WebSocket URL for the KV channel.
	const wsUrl = endpoint
		.replace(/^https:\/\//, "wss://")
		.replace(/^http:\/\//, "ws://")
		.replace(/\/$/, "");

	kvChannel = mod.connect({
		url: `${wsUrl}/kv/connect`,
		token: token ?? undefined,
		namespace,
	});
	channelDisconnected = false;

	registerShutdownHandler();

	return kvChannel;
}

/**
 * Convert binding values to JSON-compatible types for the native addon.
 * The native addon accepts serde_json::Value via napi, so bigint and
 * Uint8Array need conversion.
 */
function toNativeBindings(args: unknown[]): unknown[] {
	return args.map((arg) => {
		if (typeof arg === "bigint") {
			return Number(arg);
		}
		if (arg instanceof Uint8Array) {
			return Array.from(arg);
		}
		return arg;
	});
}

/**
 * Create a RawAccess database client backed by the native SQLite addon.
 * The KV channel is shared per process; a new database is opened per actor.
 */
export function createNativeRawAccess(actorId: string): RawAccess {
	const mod = getNativeModule();
	const channel = getOrCreateKvChannel();
	const nativeDb = mod.openDatabase(channel, actorId);
	let closed = false;
	const mutex = new AsyncMutex();

	const ensureOpen = () => {
		if (closed) {
			throw new Error("database is closed");
		}
	};

	return {
		execute: async <
			TRow extends Record<string, unknown> = Record<
				string,
				unknown
			>,
		>(
			query: string,
			...args: unknown[]
		): Promise<TRow[]> => {
			return await mutex.run(async () => {
				ensureOpen();

				if (args.length > 0) {
					// The native addon validates binding types in Rust
					// (bind_params). Convert bigint/Uint8Array to
					// JSON-compatible representations.
					const bindings = toNativeBindings(args);
					const token = query
						.trimStart()
						.slice(0, 16)
						.toUpperCase();
					const returnsRows =
						token.startsWith("SELECT") ||
						token.startsWith("PRAGMA") ||
						token.startsWith("WITH");

					if (returnsRows) {
						const { rows, columns } = await mod.query(
							nativeDb,
							query,
							bindings,
						);
						return rows.map((row: unknown[]) => {
							const rowObj: Record<string, unknown> = {};
							for (let i = 0; i < columns.length; i++) {
								rowObj[columns[i]] = row[i];
							}
							return rowObj;
						}) as TRow[];
					}

					await mod.execute(nativeDb, query, bindings);
					return [] as TRow[];
				}

				// Multi-statement SQL (e.g., migrations) without parameters.
				// Uses the native exec which loops sqlite3_prepare_v2 with
				// tail pointer tracking.
				const { rows, columns } = await mod.exec(nativeDb, query);
				return rows.map((row: unknown[]) => {
					const rowObj: Record<string, unknown> = {};
					for (let i = 0; i < columns.length; i++) {
						rowObj[columns[i]] = row[i];
					}
					return rowObj;
				}) as TRow[];
			});
		},
		close: async () => {
			await mutex.run(async () => {
				if (closed) return;
				closed = true;
				mod.closeDatabase(nativeDb);
			});
		},
	};
}
