import type { DatabaseProviderContext } from "./config";
import type { IDatabase } from "@rivetkit/sqlite-vfs";
import type { KvVfsOptions } from "@rivetkit/sqlite-vfs";
import type { ActorMetrics } from "@/actor/metrics";
import {
	binarySearch,
	type PreloadedEntries,
} from "../actor/instance/preload-map";

type ActorKvOperations = DatabaseProviderContext["kv"];
type SqliteBindings = NonNullable<Parameters<IDatabase["run"]>[1]>;
type SqliteBindingObject = Record<string, unknown>;

function isSqliteBindingValue(value: unknown): boolean {
	if (
		value === null ||
		typeof value === "number" ||
		typeof value === "string" ||
		typeof value === "bigint" ||
		value instanceof Uint8Array
	) {
		return true;
	}

	if (Array.isArray(value)) {
		return value.every((item) => typeof item === "number");
	}

	return false;
}

function isPlainObject(value: unknown): value is Record<string, unknown> {
	if (!value || typeof value !== "object" || Array.isArray(value)) {
		return false;
	}
	return Object.getPrototypeOf(value) === Object.prototype;
}

export function isSqliteBindingObject(
	value: unknown,
): value is SqliteBindingObject {
	if (!isPlainObject(value)) {
		return false;
	}

	return Object.values(value).every((entry) => isSqliteBindingValue(entry));
}

export function isSqliteBindingArray(value: unknown): value is unknown[] {
	return (
		Array.isArray(value) &&
		value.every((entry) => isSqliteBindingValue(entry))
	);
}

export function toSqliteBindings(
	input: unknown[] | SqliteBindingObject,
): SqliteBindings {
	if (Array.isArray(input)) {
		for (const value of input) {
			if (!isSqliteBindingValue(value)) {
				throw new Error(
					`unsupported sqlite binding type: ${typeof value}`,
				);
			}
		}
		return input as SqliteBindings;
	}

	if (isSqliteBindingObject(input)) {
		return input as SqliteBindings;
	}

	throw new Error("unsupported sqlite binding collection");
}

/**
 * Create a KV store wrapper that uses the actor driver's KV operations.
 * Tracks per-operation metrics when an ActorMetrics instance is provided.
 *
 * When `preloadedEntries` is provided, `get` and `getBatch` check the
 * preloaded sorted array via binary search before falling back to KV.
 * Write operations always pass through to KV unchanged.
 *
 * Call `clearPreload()` on the returned object after migrations complete
 * to release the preloaded data and free memory.
 */
export function createActorKvStore(
	kv: ActorKvOperations,
	metrics?: ActorMetrics,
	preloadedEntries?: PreloadedEntries,
): KvVfsOptions & { clearPreload: () => void; poison: () => void } {
	let preload: PreloadedEntries | undefined = preloadedEntries;
	let poisoned = false;
	const ensureNotPoisoned = () => {
		if (poisoned) {
			throw new Error(
				"Database is shutting down. A query was still in progress when the actor started stopping. Use c.abortSignal to cancel long-running work before the actor shuts down.",
			);
		}
	};

	return {
		get: async (key: Uint8Array) => {
			ensureNotPoisoned();
			// Preload hits bypass KV entirely and are not tracked in
			// kvGet metrics. Only cache misses are counted below.
			if (preload) {
				const value = binarySearch(preload, key);
				if (value !== undefined) return value;
			}
			const start = performance.now();
			const results = await kv.batchGet([key]);
			if (metrics) {
				metrics.kvGet.calls++;
				metrics.kvGet.keys++;
				metrics.kvGet.totalMs += performance.now() - start;
			}
			return results[0] ?? null;
		},
		getBatch: async (keys: Uint8Array[]) => {
			ensureNotPoisoned();
			if (!preload || keys.length === 0) {
				const start = performance.now();
				const results = await kv.batchGet(keys);
				if (metrics) {
					metrics.kvGetBatch.calls++;
					metrics.kvGetBatch.keys += keys.length;
					metrics.kvGetBatch.totalMs += performance.now() - start;
				}
				return results;
			}

			// Preload hits are not tracked in kvGetBatch metrics. Only
			// actual KV round-trips (cache misses) are counted below.
			const results: (Uint8Array | null)[] = new Array<Uint8Array | null>(
				keys.length,
			).fill(null);
			const missIndices: number[] = [];
			const missKeys: Uint8Array[] = [];

			for (let i = 0; i < keys.length; i++) {
				const value = binarySearch(preload, keys[i]);
				if (value !== undefined) {
					results[i] = value;
				} else {
					missIndices.push(i);
					missKeys.push(keys[i]);
				}
			}

			if (missKeys.length > 0) {
				const start = performance.now();
				const kvResults = await kv.batchGet(missKeys);
				if (metrics) {
					metrics.kvGetBatch.calls++;
					metrics.kvGetBatch.keys += missKeys.length;
					metrics.kvGetBatch.totalMs += performance.now() - start;
				}
				for (let i = 0; i < missIndices.length; i++) {
					results[missIndices[i]] = kvResults[i] ?? null;
				}
			}

			return results;
		},
		put: async (key: Uint8Array, value: Uint8Array) => {
			ensureNotPoisoned();
			const start = performance.now();
			await kv.batchPut([[key, value]]);
			if (metrics) {
				metrics.kvPut.calls++;
				metrics.kvPut.keys++;
				metrics.kvPut.totalMs += performance.now() - start;
			}
		},
		putBatch: async (entries: [Uint8Array, Uint8Array][]) => {
			ensureNotPoisoned();
			const start = performance.now();
			await kv.batchPut(entries);
			if (metrics) {
				metrics.kvPutBatch.calls++;
				metrics.kvPutBatch.keys += entries.length;
				metrics.kvPutBatch.totalMs += performance.now() - start;
			}
		},
		deleteBatch: async (keys: Uint8Array[]) => {
			ensureNotPoisoned();
			const start = performance.now();
			await kv.batchDelete(keys);
			if (metrics) {
				metrics.kvDeleteBatch.calls++;
				metrics.kvDeleteBatch.keys += keys.length;
				metrics.kvDeleteBatch.totalMs += performance.now() - start;
			}
		},
		clearPreload: () => {
			preload = undefined;
		},
		poison: () => {
			poisoned = true;
		},
	};
}

/**
 * Serialize async operations on a shared non-reentrant resource.
 */
export class AsyncMutex {
	#locked = false;
	#waiting: (() => void)[] = [];

	async acquire(): Promise<void> {
		while (this.#locked) {
			await new Promise<void>((resolve) => this.#waiting.push(resolve));
		}
		this.#locked = true;
	}

	release(): void {
		this.#locked = false;
		const next = this.#waiting.shift();
		if (next) {
			next();
		}
	}

	async run<T>(fn: () => Promise<T>): Promise<T> {
		await this.acquire();
		try {
			return await fn();
		} finally {
			this.release();
		}
	}
}
