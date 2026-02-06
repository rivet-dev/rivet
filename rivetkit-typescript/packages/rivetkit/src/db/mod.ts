import type { DatabaseProvider, RawAccess } from "./config";
import { getSqliteVfs } from "./sqlite-vfs";
import type { KvVfsOptions } from "./sqlite-vfs";

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
			const kvStore = createActorKvStore(ctx.kv);
			const sqliteVfs = await getSqliteVfs();
			const db = await sqliteVfs.open(ctx.actorId, kvStore);

			return {
				execute: async (query, ...args) => {
					const results: Record<string, unknown>[] = [];
					let columnNames: string[] | null = null;
					await db.exec(query, (row: unknown[], columns: string[]) => {
						// Capture column names on first row
						if (!columnNames) {
							columnNames = columns;
						}
						// Convert array row to object
						const rowObj: Record<string, unknown> = {};
						for (let i = 0; i < row.length; i++) {
							rowObj[columnNames[i]] = row[i];
						}
						results.push(rowObj);
					});
					return results;
				},
				close: async () => {
					await db.close();
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
