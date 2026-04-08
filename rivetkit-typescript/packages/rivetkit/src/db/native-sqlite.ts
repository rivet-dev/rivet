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
import type { NativeSqliteConfig, RawAccess } from "./config";
import { AsyncMutex } from "./shared";

// Type declarations for @rivetkit/sqlite-native.
// Declared inline to avoid a build-time dependency on the native addon,
// which may not be installed or compiled.

/** Typed bind parameter matching the Rust BindParam napi struct. */
export interface NativeBindParam {
	kind: "null" | "int" | "float" | "text" | "blob";
	intValue?: number;
	floatValue?: number;
	textValue?: string;
	blobValue?: Buffer;
}

export interface NativeSqliteModule {
	connect(config: {
		url: string;
		token?: string;
		namespace: string;
	}): NativeKvChannel;
	openDatabase(
		channel: NativeKvChannel,
		actorId: string,
	): Promise<NativeDatabase>;
	execute(
		db: NativeDatabase,
		sql: string,
		params?: NativeBindParam[],
	): Promise<{ changes: number }>;
	query(
		db: NativeDatabase,
		sql: string,
		params?: NativeBindParam[],
	): Promise<{ columns: string[]; rows: unknown[][] }>;
	exec(
		db: NativeDatabase,
		sql: string,
	): Promise<{ columns: string[]; rows: unknown[][] }>;
	closeDatabase(db: NativeDatabase): Promise<void>;
	disconnect(channel: NativeKvChannel): Promise<void>;
	getMetrics?(channel: NativeKvChannel): KvChannelMetricsSnapshot | undefined;
}

/** Metrics snapshot for a single KV operation type. */
export interface OpMetricsSnapshot {
	count: number;
	totalDurationUs: number;
	minDurationUs: number;
	maxDurationUs: number;
	avgDurationUs: number;
}

/** Aggregated KV channel metrics for all operation types. */
export interface KvChannelMetricsSnapshot {
	get: OpMetricsSnapshot;
	put: OpMetricsSnapshot;
	delete: OpMetricsSnapshot;
	deleteRange: OpMetricsSnapshot;
	actorOpen: OpMetricsSnapshot;
	actorClose: OpMetricsSnapshot;
}

// Opaque handles from the native addon.
export type NativeKvChannel = object;
export type NativeDatabase = object;

// Cached detection result.
let nativeModule: NativeSqliteModule | null = null;
let detectionDone = false;

// KV channels are pooled by endpoint/token/namespace so concurrent test
// runtimes do not tear down each other's connection state.
const kvChannels = new Map<string, NativeKvChannel>();

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
export async function _resetNativeDetection(disable?: boolean): Promise<void> {
	if (nativeModule) {
		const disconnectPromises = Array.from(kvChannels.values()).map(
			async (channel) => {
				try {
					await nativeModule!.disconnect(channel);
				} catch {
					// Ignore cleanup errors
				}
			},
		);
		await Promise.all(disconnectPromises);
	}
	kvChannels.clear();

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
export function getNativeModule(): NativeSqliteModule {
	if (!nativeModule) {
		throw new Error("native SQLite module not loaded");
	}
	return nativeModule;
}

/**
 * Disconnect the singleton KV channel if it exists. Safe to call multiple times.
 */
export function disconnectKvChannel(): void {
	if (nativeModule) {
		for (const channel of kvChannels.values()) {
			// Fire-and-forget the async disconnect. During process shutdown,
			// we cannot reliably await Promises (beforeExit/signal handlers
			// are synchronous). The WebSocket close frame is best-effort.
			nativeModule.disconnect(channel).catch(() => {
				// Ignore cleanup errors during shutdown.
			});
		}
	}
	kvChannels.clear();
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
function getKvChannelConfig(config?: NativeSqliteConfig) {
	const endpoint =
		config?.endpoint ?? getRivetEndpoint() ?? "http://127.0.0.1:6420";
	const token = config?.token ?? getRivetToken();
	const namespace = config?.namespace ?? getRivetNamespace() ?? "default";

	// Convert HTTP(S) endpoint to WebSocket URL for the KV channel.
	const wsUrl = endpoint
		.replace(/^https:\/\//, "wss://")
		.replace(/^http:\/\//, "ws://")
		.replace(/\/$/, "");

	return {
		wsUrl,
		token: token ?? undefined,
		namespace,
		key: `${wsUrl}\u0000${token ?? ""}\u0000${namespace}`,
	};
}

export function getOrCreateKvChannel(
	config?: NativeSqliteConfig,
): NativeKvChannel {
	const mod = getNativeModule();
	const channelConfig = getKvChannelConfig(config);
	const existing = kvChannels.get(channelConfig.key);
	if (existing) return existing;

	const channel = mod.connect({
		url: channelConfig.wsUrl,
		token: channelConfig.token,
		namespace: channelConfig.namespace,
	});
	kvChannels.set(channelConfig.key, channel);

	registerShutdownHandler();

	return channel;
}

function toNativeBinding(arg: unknown): NativeBindParam {
	if (arg === null || arg === undefined) {
		return { kind: "null" };
	}
	if (typeof arg === "bigint") {
		return { kind: "int", intValue: Number(arg) };
	}
	if (typeof arg === "number") {
		if (Number.isInteger(arg)) {
			return { kind: "int", intValue: arg };
		}
		return { kind: "float", floatValue: arg };
	}
	if (typeof arg === "string") {
		return { kind: "text", textValue: arg };
	}
	if (typeof arg === "boolean") {
		return { kind: "int", intValue: arg ? 1 : 0 };
	}
	if (arg instanceof Uint8Array) {
		return { kind: "blob", blobValue: Buffer.from(arg) };
	}
	throw new Error(`unsupported bind parameter type: ${typeof arg}`);
}

/**
 * Convert binding values to typed BindParam objects for the native addon.
 * Uses Buffer for blobs instead of JSON arrays to avoid 20x serialization
 * overhead. See docs-internal/engine/NATIVE_SQLITE_REVIEW_FIXES.md M7.
 */
export function toNativeBindings(args: unknown[]): NativeBindParam[] {
	return args.map((arg): NativeBindParam => {
		return toNativeBinding(arg);
	});
}

function toNativeNamedBindings(
	sql: string,
	bindings: Record<string, unknown>,
): NativeBindParam[] {
	const orderedNames = extractNamedSqliteParameters(sql);
	if (orderedNames.length === 0) {
		return toNativeBindings(Object.values(bindings));
	}

	return orderedNames.map((name) => {
		const value = getNamedSqliteBinding(bindings, name);
		if (value === undefined) {
			throw new Error(`missing bind parameter: ${name}`);
		}
		return toNativeBinding(value);
	});
}

function extractNamedSqliteParameters(sql: string): string[] {
	const orderedNames: string[] = [];
	const seen = new Set<string>();
	const pattern = /([:@$][A-Za-z_][A-Za-z0-9_]*)/g;
	for (const match of sql.matchAll(pattern)) {
		const name = match[1];
		if (seen.has(name)) {
			continue;
		}
		seen.add(name);
		orderedNames.push(name);
	}
	return orderedNames;
}

function getNamedSqliteBinding(
	bindings: Record<string, unknown>,
	name: string,
): unknown {
	if (name in bindings) {
		return bindings[name];
	}

	const bareName = name.slice(1);
	if (bareName in bindings) {
		return bindings[bareName];
	}

	for (const prefix of [":", "@", "$"] as const) {
		const candidate = `${prefix}${bareName}`;
		if (candidate in bindings) {
			return bindings[candidate];
		}
	}

	return undefined;
}

/**
 * Get a snapshot of KV channel operation metrics.
 * Returns undefined if the native module is not available or the channel is not connected.
 */
export function getKvChannelMetrics(): KvChannelMetricsSnapshot | undefined {
	if (!nativeModule?.getMetrics) return undefined;
	const channel = kvChannels.get(getKvChannelConfig().key);
	if (!channel) return undefined;
	return nativeModule.getMetrics(channel) as KvChannelMetricsSnapshot | undefined;
}

/**
 * Disconnect the KV channel for the current endpoint/token/namespace only.
 * This is used by the local driver test harness so one test runtime does not
 * shut down another concurrent runtime's native SQLite channel.
 */
export async function disconnectKvChannelForCurrentConfig(
	config?: NativeSqliteConfig,
): Promise<number> {
	if (!nativeModule) {
		return 0;
	}

	const { key } = getKvChannelConfig(config);
	const channel = kvChannels.get(key);
	if (!channel) {
		return 0;
	}

	kvChannels.delete(key);
	await nativeModule.disconnect(channel);
	return 1;
}

/**
 * Disconnect a specific KV channel instance and only clear the cached entry
 * if it still points at that same channel.
 */
export async function disconnectKvChannelIfCurrent(
	channel: NativeKvChannel,
	config?: NativeSqliteConfig,
): Promise<void> {
	if (!nativeModule) {
		return;
	}

	const { key } = getKvChannelConfig(config);
	if (kvChannels.get(key) === channel) {
		kvChannels.delete(key);
	}

	await nativeModule.disconnect(channel);
}

/**
 * Create a RawAccess database client backed by the native SQLite addon.
 * The KV channel is shared per process; a new database is opened per actor.
 */
export async function createNativeRawAccess(
	actorId: string,
	config?: NativeSqliteConfig,
): Promise<RawAccess> {
	const mod = getNativeModule();
	const channel = getOrCreateKvChannel(config);
	const nativeDb = await mod.openDatabase(channel, actorId);
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
					const bindings =
						args.length === 1 &&
						args[0] !== null &&
						typeof args[0] === "object" &&
						!Array.isArray(args[0]) &&
						!(args[0] instanceof Uint8Array)
							? toNativeNamedBindings(
									query,
									args[0] as Record<string, unknown>,
								)
							: toNativeBindings(args);
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
				await mod.closeDatabase(nativeDb);
			});
		},
	};
}
