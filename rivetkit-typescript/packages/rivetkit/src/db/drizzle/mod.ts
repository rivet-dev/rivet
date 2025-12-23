import {
	type BetterSQLite3Database,
	drizzle as sqliteDrizzle,
} from "drizzle-orm/better-sqlite3";
import { drizzle as durableDrizzle } from "drizzle-orm/durable-sqlite";
import { migrate as durableMigrate } from "drizzle-orm/durable-sqlite/migrator";
import type { KvVfsOptions } from "../vfs/mod";
import type { DatabaseProvider, RawAccess } from "../config";

export * from "drizzle-orm/sqlite-core";

import { type Config, defineConfig as originalDefineConfig } from "drizzle-kit";

export function defineConfig(
	config: Partial<Config & { driver: "durable-sqlite" }>,
): Config {
	return originalDefineConfig({
		dialect: "sqlite",
		driver: "durable-sqlite",
		...config,
	});
}

interface DatabaseFactoryConfig<
	TSchema extends Record<string, unknown> = Record<string, never>,
> {
	schema?: TSchema;
	migrations?: any;
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

export function db<
	TSchema extends Record<string, unknown> = Record<string, never>,
>(
	config?: DatabaseFactoryConfig<TSchema>,
): DatabaseProvider<BetterSQLite3Database<TSchema> & RawAccess> {
	return {
		createClient: async (ctx) => {
			// Check if override is provided
			const override = ctx.overrideDrizzleDatabaseClient
				? await ctx.overrideDrizzleDatabaseClient()
				: undefined;

			if (override) {
				// Use the override (wrap with Drizzle)
				const client = durableDrizzle<TSchema, any>(override, config);

				return Object.assign(client, {
					execute: async (query, ...args) => {
						return client.$client.exec(query, ...args);
					},
					close: async () => {
						// Override clients don't need cleanup
					},
				} satisfies RawAccess);
			}

			// Construct KV-backed client using actor driver's KV operations
			if (!ctx.sqliteVfs) {
				throw new Error("SqliteVfs instance not provided in context. The driver must provide a sqliteVfs instance.");
			}

			const kvStore = createActorKvStore(ctx.kv);
			const db = await ctx.sqliteVfs.open(ctx.actorId, kvStore);

			// Wrap the KV-backed client with Drizzle
			const rawClient = {
				exec: async (query: string, ...args: unknown[]) => {
					await db.exec(query);
					return [];
				},
			};

			const client = durableDrizzle<TSchema, any>(rawClient, config);

			return Object.assign(client, {
				execute: async (query, ...args) => {
					return client.$client.exec(query, ...args);
				},
				close: async () => {
					await db.close();
				},
			} satisfies RawAccess);
		},
		onMigrate: async (client) => {
			if (config?.migrations) {
				await durableMigrate(client, config?.migrations);
			}
		},
		onDestroy: async (client) => {
			await client.close();
		},
	};
}
