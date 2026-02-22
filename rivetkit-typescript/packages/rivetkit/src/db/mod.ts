import type { KvVfsOptions } from "./sqlite-vfs";
import type { DatabaseProvider, RawAccess } from "./config";

interface DatabaseFactoryConfig {
	onMigrate?: (db: RawAccess) => Promise<void> | void;
}

/**
 * Mutex to serialize async operations on a wa-sqlite database handle.
 * wa-sqlite is not safe for concurrent operations on the same handle.
 */
class DbMutex {
	#locked = false;
	#waiting: (() => void)[] = [];

	async run<T>(fn: () => Promise<T>): Promise<T> {
		while (this.#locked) {
			await new Promise<void>((resolve) => this.#waiting.push(resolve));
		}
		this.#locked = true;
		try {
			return await fn();
		} finally {
			this.#locked = false;
			const next = this.#waiting.shift();
			if (next) {
				next();
			}
		}
	}
}

/**
 * Create a KV store wrapper that uses the actor driver's KV operations
 */
function createActorKvStore(kv: {
	batchPut: (entries: [Uint8Array, Uint8Array][]) => Promise<void>;
	batchGet: (keys: Uint8Array[]) => Promise<(Uint8Array | null)[]>;
	batchDelete: (keys: Uint8Array[]) => Promise<void>;
}): KvVfsOptions {
	return {
		get: async (key: Uint8Array) => {
			const results = await kv.batchGet([key]);
			return results[0];
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

export function db({
	onMigrate,
}: DatabaseFactoryConfig = {}): DatabaseProvider<RawAccess> {
	const mutex = new DbMutex();

	return {
		createClient: async (ctx) => {
			// Check if override is provided
			const override = ctx.overrideRawDatabaseClient
				? await ctx.overrideRawDatabaseClient()
				: undefined;

			if (override) {
				// Use the override
				return {
					execute: async <
						TRow extends Record<string, unknown> = Record<string, unknown>,
					>(
						query: string,
						...args: unknown[]
					): Promise<TRow[]> => {
						return await override.exec<TRow>(query, ...args);
					},
					close: async () => {
						// Override clients don't need cleanup
					},
				} satisfies RawAccess;
			}

			// Construct KV-backed client using actor driver's KV operations
			if (!ctx.sqliteVfs) {
				throw new Error("SqliteVfs instance not provided in context. The driver must provide a sqliteVfs instance.");
			}

			const kvStore = createActorKvStore(ctx.kv);
			const db = await ctx.sqliteVfs.open(ctx.actorId, kvStore);
			let closed = false;
			const ensureOpen = () => {
				if (closed) {
					throw new Error("database is closed");
				}
			};

			return {
				execute: async <
					TRow extends Record<string, unknown> = Record<string, unknown>,
					>(
						query: string,
						...args: unknown[]
					): Promise<TRow[]> => {
						return mutex.run(async () => {
							ensureOpen();

							if (args.length > 0) {
								// Use parameterized query when args are provided
								const { rows, columns } = await db.query(query, args);
								return rows.map((row: unknown[]) => {
									const rowObj: Record<string, unknown> = {};
									for (let i = 0; i < row.length; i++) {
										rowObj[columns[i]] = row[i];
									}
									return rowObj;
								}) as TRow[];
							}

							// Use exec for non-parameterized queries
							const results: Record<string, unknown>[] = [];
							let columnNames: string[] | null = null;
							await db.exec(query, (row: unknown[], columns: string[]) => {
								if (!columnNames) {
									columnNames = columns;
								}
								const rowObj: Record<string, unknown> = {};
								for (let i = 0; i < row.length; i++) {
									rowObj[columnNames[i]] = row[i];
								}
								results.push(rowObj);
							});
							return results as TRow[];
						});
					},
				close: async () => {
					await mutex.run(async () => {
						if (closed) {
							return;
						}
						closed = true;
						await db.close();
					});
				},
			} satisfies RawAccess;
		},
		onMigrate: async (client) => {
			if (onMigrate) {
				await onMigrate(client);
			}
		},
		onDestroy: async (client) => {
			await client.close();
		},
	};
}
