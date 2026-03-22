import type { DatabaseProviderContext } from "./config";
import type { IDatabase } from "@rivetkit/sqlite-vfs";
import type { KvVfsOptions } from "@rivetkit/sqlite-vfs";
import type { ActorMetrics } from "@/actor/metrics";

type ActorKvOperations = DatabaseProviderContext["kv"];
type SqliteBindings = NonNullable<Parameters<IDatabase["run"]>[1]>;

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

export function toSqliteBindings(args: unknown[]): SqliteBindings {
	for (const value of args) {
		if (!isSqliteBindingValue(value)) {
			throw new Error(`unsupported sqlite binding type: ${typeof value}`);
		}
	}

	return args as SqliteBindings;
}

/**
 * Create a KV store wrapper that uses the actor driver's KV operations.
 * Tracks per-operation metrics when an ActorMetrics instance is provided.
 */
export function createActorKvStore(
	kv: ActorKvOperations,
	metrics?: ActorMetrics,
): KvVfsOptions {
	return {
		get: async (key: Uint8Array) => {
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
			const start = performance.now();
			const results = await kv.batchGet(keys);
			if (metrics) {
				metrics.kvGetBatch.calls++;
				metrics.kvGetBatch.keys += keys.length;
				metrics.kvGetBatch.totalMs += performance.now() - start;
			}
			return results;
		},
		put: async (key: Uint8Array, value: Uint8Array) => {
			const start = performance.now();
			await kv.batchPut([[key, value]]);
			if (metrics) {
				metrics.kvPut.calls++;
				metrics.kvPut.keys++;
				metrics.kvPut.totalMs += performance.now() - start;
			}
		},
		putBatch: async (entries: [Uint8Array, Uint8Array][]) => {
			const start = performance.now();
			await kv.batchPut(entries);
			if (metrics) {
				metrics.kvPutBatch.calls++;
				metrics.kvPutBatch.keys += entries.length;
				metrics.kvPutBatch.totalMs += performance.now() - start;
			}
		},
		deleteBatch: async (keys: Uint8Array[]) => {
			const start = performance.now();
			await kv.batchDelete(keys);
			if (metrics) {
				metrics.kvDeleteBatch.calls++;
				metrics.kvDeleteBatch.keys += keys.length;
				metrics.kvDeleteBatch.totalMs += performance.now() - start;
			}
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
