/**
 * SQLite VFS Pool - shares WASM SQLite instances across actors to reduce
 * memory overhead. Instead of one WASM module per actor, multiple actors
 * share a single instance, with short file names routing to separate KV
 * namespaces.
 */

import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import path from "node:path";
import { SqliteVfs } from "./vfs";
import type { ISqliteVfs, IDatabase } from "./vfs";
import type { KvVfsOptions } from "./types";

function createNodeRequire(): NodeJS.Require {
	return createRequire(
		path.join(process.cwd(), "__rivetkit_sqlite_require__.cjs"),
	);
}

export interface SqliteVfsPoolConfig {
	actorsPerInstance: number;
	idleDestroyMs?: number;
}

/**
 * Internal state for a single WASM SQLite instance shared by multiple actors.
 */
interface PoolInstance {
	vfs: SqliteVfs;
	/** Actor IDs currently assigned to this instance. */
	actors: Set<string>;
	/** Monotonically increasing counter for generating short file names. */
	shortNameCounter: number;
	/** Maps actorId to the short name assigned within this instance. */
	actorShortNames: Map<string, string>;
	/** Short names released by actors that closed successfully, available for reuse. */
	availableShortNames: Set<string>;
	/** Short names that failed to close cleanly. Not reused until instance is destroyed. */
	poisonedShortNames: Set<string>;
	/** Number of in-flight operations (e.g. open calls) on this instance. */
	opsInFlight: number;
	/** Handle for the idle destruction timer, or null if not scheduled. */
	idleTimer: ReturnType<typeof setTimeout> | null;
	/** True once destruction has started. Prevents double-destroy. */
	destroying: boolean;
}

/**
 * Manages a pool of SqliteVfs instances, assigning actors to instances using
 * bin-packing to maximize density. The WASM module is compiled once and
 * reused across all instances.
 */
export class SqliteVfsPool {
	readonly #config: SqliteVfsPoolConfig;
	#modulePromise: Promise<WebAssembly.Module> | null = null;
	readonly #instances: Set<PoolInstance> = new Set();
	readonly #actorToInstance: Map<string, PoolInstance> = new Map();
	readonly #actorToHandle: Map<string, PooledSqliteHandle> = new Map();
	#shuttingDown = false;

	constructor(config: SqliteVfsPoolConfig) {
		if (
			!Number.isInteger(config.actorsPerInstance) ||
			config.actorsPerInstance < 1
		) {
			throw new Error(
				`actorsPerInstance must be a positive integer, got ${config.actorsPerInstance}`,
			);
		}
		this.#config = config;
	}

	/**
	 * Compile the WASM module once and cache the promise. Subsequent calls
	 * return the same promise, avoiding redundant compilation.
	 */
	#getModule(): Promise<WebAssembly.Module> {
		if (!this.#modulePromise) {
			this.#modulePromise = (async () => {
				const require = createNodeRequire();
				const wasmPath = require.resolve(
					"@rivetkit/sqlite/dist/wa-sqlite-async.wasm",
				);
				const wasmBinary = readFileSync(wasmPath);
				return WebAssembly.compile(wasmBinary);
			})();
			// Clear the cached promise on rejection so subsequent calls retry
			// compilation instead of returning the same rejected promise forever.
			this.#modulePromise.catch(() => {
				this.#modulePromise = null;
			});
		}
		return this.#modulePromise;
	}

	/** Number of live WASM instances in the pool. */
	get instanceCount(): number {
		return this.#instances.size;
	}

	/** Number of actors currently assigned to pool instances. */
	get actorCount(): number {
		return this.#actorToInstance.size;
	}

	/**
	 * Acquire a pooled VFS handle for the given actor. Returns a
	 * PooledSqliteHandle with sticky assignment. If the actor is already
	 * assigned, the existing handle is returned.
	 *
	 * Bin-packing: picks the instance with the most actors that still has
	 * capacity. If all instances are full, creates a new one using the
	 * cached WASM module.
	 */
	async acquire(actorId: string): Promise<PooledSqliteHandle> {
		if (this.#shuttingDown) {
			throw new Error("SqliteVfsPool is shutting down");
		}

		// Sticky assignment: return existing handle.
		const existingHandle = this.#actorToHandle.get(actorId);
		if (existingHandle) {
			return existingHandle;
		}

		// Bin-packing: pick instance with most actors that still has capacity.
		// Skip instances that are being destroyed.
		let bestInstance: PoolInstance | null = null;
		let bestCount = -1;
		for (const instance of this.#instances) {
			if (instance.destroying) continue;
			const count = instance.actors.size;
			if (count < this.#config.actorsPerInstance && count > bestCount) {
				bestInstance = instance;
				bestCount = count;
			}
		}

		// If all instances are full, compile the module and re-check capacity.
		// Multiple concurrent acquire() calls may all reach this point. After
		// awaiting the module, re-scan for capacity that another caller may
		// have created during the await, to avoid creating duplicate instances.
		if (!bestInstance) {
			const wasmModule = await this.#getModule();
			if (this.#shuttingDown) {
				throw new Error("SqliteVfsPool is shutting down");
			}

			// Re-check sticky assignment: another concurrent acquire() for the
			// same actorId may have completed during the await.
			const existingHandleAfterAwait = this.#actorToHandle.get(actorId);
			if (existingHandleAfterAwait) {
				return existingHandleAfterAwait;
			}

			// Re-scan for an instance with available capacity that was created
			// by another concurrent acquire() during the module compilation.
			for (const instance of this.#instances) {
				if (instance.destroying) continue;
				const count = instance.actors.size;
				if (
					count < this.#config.actorsPerInstance &&
					count > bestCount
				) {
					bestInstance = instance;
					bestCount = count;
				}
			}

			if (!bestInstance) {
				const vfs = new SqliteVfs(wasmModule);
				bestInstance = {
					vfs,
					actors: new Set(),
					shortNameCounter: 0,
					actorShortNames: new Map(),
					availableShortNames: new Set(),
					poisonedShortNames: new Set(),
					opsInFlight: 0,
					idleTimer: null,
					destroying: false,
				};
				this.#instances.add(bestInstance);
			}
		}

		// Cancel idle timer synchronously since this instance is getting a
		// new actor and should not be destroyed.
		this.#cancelIdleTimer(bestInstance);

		// Assign actor to instance with a short file name. Prefer recycled
		// names from the available set before generating a new one.
		let shortName: string;
		const recycled = bestInstance.availableShortNames.values().next();
		if (!recycled.done) {
			shortName = recycled.value;
			bestInstance.availableShortNames.delete(shortName);
		} else {
			shortName = String(bestInstance.shortNameCounter++);
		}
		bestInstance.actors.add(actorId);
		bestInstance.actorShortNames.set(actorId, shortName);
		this.#actorToInstance.set(actorId, bestInstance);

		const handle = new PooledSqliteHandle(shortName, actorId, this);
		this.#actorToHandle.set(actorId, handle);

		return handle;
	}

	/**
	 * Release an actor's assignment from the pool. Force-closes all database
	 * handles for the actor, recycles or poisons the short name, and
	 * decrements the instance refcount.
	 */
	async release(actorId: string): Promise<void> {
		const instance = this.#actorToInstance.get(actorId);
		if (!instance) {
			return;
		}

		const shortName = instance.actorShortNames.get(actorId);
		if (shortName === undefined) {
			return;
		}

		// Force-close all Database handles for this actor's short name.
		const { allSucceeded } =
			await instance.vfs.forceCloseByFileName(shortName);

		if (allSucceeded) {
			instance.availableShortNames.add(shortName);
		} else {
			instance.poisonedShortNames.add(shortName);
		}

		// Remove actor from instance tracking.
		instance.actors.delete(actorId);
		instance.actorShortNames.delete(actorId);
		this.#actorToInstance.delete(actorId);
		this.#actorToHandle.delete(actorId);

		// Start idle timer if instance has no actors and no in-flight ops.
		// Skip if shutting down to avoid leaking timers after shutdown
		// completes.
		if (
			instance.actors.size === 0 &&
			instance.opsInFlight === 0 &&
			!this.#shuttingDown
		) {
			this.#startIdleTimer(instance);
		}
	}

	/**
	 * Track an in-flight operation on an instance. Increments opsInFlight
	 * before running fn, decrements after using try/finally to prevent
	 * drift from exceptions. If the decrement brings opsInFlight to 0
	 * with refcount also 0, starts the idle timer.
	 */
	async #trackOp<T>(
		instance: PoolInstance,
		fn: () => Promise<T>,
	): Promise<T> {
		instance.opsInFlight++;
		try {
			return await fn();
		} finally {
			instance.opsInFlight--;
			if (
				instance.actors.size === 0 &&
				instance.opsInFlight === 0 &&
				!instance.destroying &&
				!this.#shuttingDown
			) {
				this.#startIdleTimer(instance);
			}
		}
	}

	/**
	 * Open a database on behalf of an actor, tracked as an in-flight
	 * operation. Used by PooledSqliteHandle to avoid exposing PoolInstance.
	 */
	async openForActor(
		actorId: string,
		shortName: string,
		options: KvVfsOptions,
	): Promise<IDatabase> {
		const instance = this.#actorToInstance.get(actorId);
		if (!instance) {
			throw new Error(
				`Actor ${actorId} is not assigned to any pool instance`,
			);
		}
		return this.#trackOp(instance, () =>
			instance.vfs.open(shortName, options),
		);
	}

	/**
	 * Track an in-flight database operation for the given actor. Resolves the
	 * actor's pool instance and wraps the operation with opsInFlight tracking.
	 * If the actor has already been released, the operation runs without
	 * tracking since the instance may already be destroyed.
	 */
	async trackOpForActor<T>(
		actorId: string,
		fn: () => Promise<T>,
	): Promise<T> {
		const instance = this.#actorToInstance.get(actorId);
		if (!instance) {
			return fn();
		}
		return this.#trackOp(instance, fn);
	}

	#startIdleTimer(instance: PoolInstance): void {
		if (instance.idleTimer || instance.destroying) return;
		const idleDestroyMs = this.#config.idleDestroyMs ?? 30_000;
		instance.idleTimer = setTimeout(() => {
			instance.idleTimer = null;
			// Check opsInFlight in addition to actors.size. With tracked
			// database operations (TrackedDatabase), opsInFlight can be >0
			// while actors.size is 0 if the last operation is still in-flight
			// after release. The #trackOp finally block will re-start the
			// idle timer when ops drain to 0.
			if (
				instance.actors.size === 0 &&
				instance.opsInFlight === 0 &&
				!instance.destroying
			) {
				this.#destroyInstance(instance);
			}
		}, idleDestroyMs);
	}

	#cancelIdleTimer(instance: PoolInstance): void {
		if (instance.idleTimer) {
			clearTimeout(instance.idleTimer);
			instance.idleTimer = null;
		}
	}

	async #destroyInstance(instance: PoolInstance): Promise<void> {
		instance.destroying = true;
		this.#cancelIdleTimer(instance);
		// Remove from pool map first so no new actors can be assigned.
		this.#instances.delete(instance);
		try {
			await instance.vfs.forceCloseAll();
			await instance.vfs.destroy();
		} catch (error) {
			console.warn("SqliteVfsPool: failed to destroy instance", error);
		}
	}

	/**
	 * Graceful shutdown. Rejects new acquire() calls, cancels idle timers,
	 * force-closes all databases, destroys all VFS instances, and clears pool
	 * state.
	 */
	async shutdown(): Promise<void> {
		this.#shuttingDown = true;

		// Snapshot instances to array since we mutate the set during iteration.
		const instances = [...this.#instances];

		for (const instance of instances) {
			this.#cancelIdleTimer(instance);
			this.#instances.delete(instance);

			// Check for in-flight operations (e.g. a concurrent release() call
			// mid-forceCloseByFileName). Database.close() is idempotent
			// (US-019), so concurrent close from shutdown + release is safe,
			// but log a warning for observability.
			if (instance.opsInFlight > 0) {
				console.warn(
					`SqliteVfsPool: shutting down instance with ${instance.opsInFlight} in-flight operation(s). ` +
						"Concurrent close is safe due to Database.close() idempotency.",
				);
			}

			try {
				await instance.vfs.forceCloseAll();
				await instance.vfs.destroy();
			} catch (error) {
				console.warn(
					"SqliteVfsPool: failed to destroy instance during shutdown",
					error,
				);
			}
		}

		this.#actorToInstance.clear();
		this.#actorToHandle.clear();
	}
}

/**
 * Wraps a Database with opsInFlight tracking so the pool's idle timer
 * does not destroy instances while database operations are in-flight.
 * The unwrapped Database remains in SqliteVfs's #openDatabases set
 * for force-close purposes.
 */
class TrackedDatabase implements IDatabase {
	readonly #inner: IDatabase;
	readonly #pool: SqliteVfsPool;
	readonly #actorId: string;

	constructor(inner: IDatabase, pool: SqliteVfsPool, actorId: string) {
		this.#inner = inner;
		this.#pool = pool;
		this.#actorId = actorId;
	}

	async exec(
		...args: Parameters<IDatabase["exec"]>
	): ReturnType<IDatabase["exec"]> {
		return this.#pool.trackOpForActor(this.#actorId, () =>
			this.#inner.exec(...args),
		);
	}

	async run(
		...args: Parameters<IDatabase["run"]>
	): ReturnType<IDatabase["run"]> {
		return this.#pool.trackOpForActor(this.#actorId, () =>
			this.#inner.run(...args),
		);
	}

	async query(
		...args: Parameters<IDatabase["query"]>
	): ReturnType<IDatabase["query"]> {
		return this.#pool.trackOpForActor(this.#actorId, () =>
			this.#inner.query(...args),
		);
	}

	async close(): ReturnType<IDatabase["close"]> {
		return this.#pool.trackOpForActor(this.#actorId, () =>
			this.#inner.close(),
		);
	}

	get fileName(): string {
		return this.#inner.fileName;
	}
}

/**
 * A pooled VFS handle for a single actor. Implements ISqliteVfs so callers
 * can use it interchangeably with a standalone SqliteVfs. The short name
 * assigned by the pool is used as the VFS file path, while the caller's
 * KvVfsOptions routes data to the correct KV namespace.
 */
export class PooledSqliteHandle implements ISqliteVfs {
	readonly #shortName: string;
	readonly #actorId: string;
	readonly #pool: SqliteVfsPool;
	#released = false;

	constructor(shortName: string, actorId: string, pool: SqliteVfsPool) {
		this.#shortName = shortName;
		this.#actorId = actorId;
		this.#pool = pool;
	}

	/**
	 * Open a database on the shared instance. Uses the pool-assigned short
	 * name as the VFS file path, with the caller's KvVfsOptions for KV
	 * routing. The open call itself is tracked as an in-flight operation,
	 * and the returned Database is wrapped so that exec(), run(), query(),
	 * and close() are also tracked via opsInFlight.
	 */
	async open(_fileName: string, options: KvVfsOptions): Promise<IDatabase> {
		if (this.#released) {
			throw new Error("PooledSqliteHandle has been released");
		}
		const db = await this.#pool.openForActor(
			this.#actorId,
			this.#shortName,
			options,
		);
		return new TrackedDatabase(db, this.#pool, this.#actorId);
	}

	/**
	 * Release this actor's assignment back to the pool. Idempotent: calling
	 * destroy() more than once is a no-op, preventing double-release from
	 * decrementing the instance refcount below actual.
	 */
	async destroy(): Promise<void> {
		if (this.#released) {
			return;
		}
		this.#released = true;
		await this.#pool.release(this.#actorId);
	}
}
