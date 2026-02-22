import type { KvVfsOptions } from "./sqlite-vfs";
import type { DatabaseProvider, RawAccess } from "./config";

export type { RawAccess } from "./config";

interface DatabaseFactoryConfig {
	onMigrate?: (db: RawAccess) => Promise<void> | void;
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
			let op: Promise<void> = Promise.resolve();

			const serialize = async <T>(fn: () => Promise<T>): Promise<T> => {
				// Ensure wa-sqlite calls are not concurrent. Actors can process multiple
				// actions concurrently, and wa-sqlite is not re-entrant.
				const next = op.then(fn, fn);
				op = next.then(
					() => undefined,
					() => undefined,
				);
				return await next;
			};

			return {
				execute: async <
					TRow extends Record<string, unknown> = Record<string, unknown>,
				>(
					query: string,
					...args: unknown[]
				): Promise<TRow[]> => {
					return await serialize(async () => {
						ensureOpen();

						// `db.exec` does not support binding `?` placeholders.
						// Use `db.query` for statements that return rows and `db.run` for
						// statements that mutate data when parameters are provided.
						// Keep using `db.exec` for non-parameterized SQL because it
						// supports multi-statement migrations.
						if (args.length > 0) {
							const token = query.trimStart().slice(0, 16).toUpperCase();
							const returnsRows =
								token.startsWith("SELECT") ||
								token.startsWith("PRAGMA") ||
								token.startsWith("WITH");

							if (returnsRows) {
								const { rows, columns } = await db.query(query, args);
								return rows.map((row: unknown[]) => {
									const rowObj: Record<string, unknown> = {};
									for (let i = 0; i < columns.length; i++) {
										rowObj[columns[i]] = row[i];
									}
									return rowObj;
								}) as TRow[];
							}

							await db.run(query, args);
							return [] as TRow[];
						}

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
					await serialize(async () => {
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
