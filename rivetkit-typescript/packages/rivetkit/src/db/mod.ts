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
					execute: async (query, ...args) => {
						return override.exec(query, ...args);
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
			let op = Promise.resolve();

			const serialize = async <T>(fn: () => Promise<T>): Promise<T> => {
				// Ensure wa-sqlite calls are not concurrent. Actors can process multiple
				// actions concurrently, and wa-sqlite is not re-entrant.
				const next = op.then(fn, fn);
				op = next.then(
					() => undefined,
					() => undefined,
				);
				return next;
			};

			return {
				execute: async (query, ...args) => {
					return await serialize(async () => {
						// `db.exec` does not support binding `?` placeholders.
						//
						// When parameters are provided:
						// - Use `db.query` for statements that return rows (SELECT/PRAGMA/WITH).
						// - Use `db.run` for DML statements (INSERT/UPDATE/DELETE, etc).
						//
						// When no parameters are provided, keep using `db.exec` because it supports
						// multiple statements (useful for migrations).
						if (args.length > 0) {
							const token = query.trimStart().slice(0, 16).toUpperCase();
							const returnsRows =
								token.startsWith("SELECT") ||
								token.startsWith("PRAGMA") ||
								token.startsWith("WITH");

							if (returnsRows) {
								const { rows, columns } = await db.query(query, args);
								return rows.map((row) => {
									const rowObj: Record<string, unknown> = {};
									for (let i = 0; i < columns.length; i++) {
										rowObj[columns[i]] = row[i];
									}
									return rowObj;
								});
							}

							await db.run(query, args);
							return [];
						}

						const results: Record<string, unknown>[] = [];
						let columnNames: string[] | null = null;
						await db.exec(query, (row: unknown[], columns: string[]) => {
							if (!columnNames) columnNames = columns;
							const rowObj: Record<string, unknown> = {};
							for (let i = 0; i < row.length; i++) {
								rowObj[columnNames[i]] = row[i];
							}
							results.push(rowObj);
						});
						return results;
					});
				},
				close: async () => {
					await serialize(async () => {
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
