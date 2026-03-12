import type { DatabaseProviderContext } from "./config";
import type { Database } from "@rivetkit/sqlite-vfs";
import type { KvVfsOptions } from "./sqlite-vfs";

type ActorKvOperations = DatabaseProviderContext["kv"];
type SqliteBindings = NonNullable<Parameters<Database["run"]>[1]>;
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
 */
export function createActorKvStore(kv: ActorKvOperations): KvVfsOptions {
	return {
		get: async (key: Uint8Array) => {
			const results = await kv.batchGet([key]);
			return results[0] ?? null;
		},
		getBatch: async (keys: Uint8Array[]) => {
			return await kv.batchGet(keys);
		},
		put: async (key: Uint8Array, value: Uint8Array) => {
			await kv.batchPut([[key, value]]);
		},
		putBatch: async (entries: [Uint8Array, Uint8Array][]) => {
			await kv.batchPut(entries);
		},
		deleteBatch: async (keys: Uint8Array[]) => {
			await kv.batchDelete(keys);
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
