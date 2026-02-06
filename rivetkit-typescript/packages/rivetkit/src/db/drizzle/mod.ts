import {
	type BetterSQLite3Database,
	drizzle as sqliteDrizzle,
} from "drizzle-orm/better-sqlite3";
import { drizzle as durableDrizzle } from "drizzle-orm/durable-sqlite";
import { migrate as durableMigrate } from "drizzle-orm/durable-sqlite/migrator";
import type { DatabaseProvider, RawAccess } from "../config";
import { getSqliteVfs } from "../sqlite-vfs";
import type { KvVfsOptions } from "../sqlite-vfs";

export * from "drizzle-orm/sqlite-core";

import { type Config, defineConfig as originalDefineConfig } from "drizzle-kit";

type MigrationConfig = {
	journal: {
		entries: { idx: number; when: number; tag: string; breakpoints: boolean }[];
	};
	migrations: Record<string, string>;
};

function getMigrationStatements(config: MigrationConfig): string[] {
	const statements: string[] = [];
	for (const entry of config.journal.entries) {
		const key = `m${entry.idx.toString().padStart(4, "0")}`;
		const sql = config.migrations[key];
		if (!sql) {
			throw new Error(`Missing migration: ${entry.tag}`);
		}
		const parts = sql
			.split("--> statement-breakpoint")
			.map((part) => part.trim())
			.filter((part) => part.length > 0);
		statements.push(...parts);
	}
	return statements;
}

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
				const client = override as any;
				const rawClient = client.$client ?? client;
				const execute = async (query: string) => {
					const trimmed = query.trim().toUpperCase();
					if (
						rawClient?.prepare &&
						(trimmed.startsWith("SELECT") ||
							trimmed.startsWith("PRAGMA"))
					) {
						return rawClient.prepare(query).all();
					}

					if (rawClient?.exec) {
						rawClient.exec(query);
						return [];
					}

					if (rawClient?.prepare) {
						rawClient.prepare(query).run();
						return [];
					}

					if (rawClient?.run) {
						rawClient.run(query);
						return [];
					}

					if (rawClient?.all) {
						return rawClient.all(query);
					}

					throw new Error(
						"Unsupported Drizzle override database client",
					);
				};

				return Object.assign(client, {
					execute,
					close: async () => {
						if (rawClient?.close) {
							rawClient.close();
						}
					},
				} satisfies RawAccess);
			}

			// Construct KV-backed client using actor driver's KV operations
			const kvStore = createActorKvStore(ctx.kv);
			const sqliteVfs = await getSqliteVfs();
			const db = await sqliteVfs.open(ctx.actorId, kvStore);
			const executeSqlite = async (query: string) => {
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
				return results;
			};

			// Wrap the KV-backed client with Drizzle
			const rawClient = {
				exec: async (query: string, ...args: unknown[]) => {
					await db.exec(query);
					return [];
				},
			};

			const client = durableDrizzle<TSchema, any>(rawClient, config);

			return Object.assign(client, {
				execute: async (query) => {
					return executeSqlite(query);
				},
				close: async () => {
					await db.close();
				},
			} satisfies RawAccess);
		},
		onMigrate: async (client) => {
			if (config?.migrations) {
				const rawClient = (client as any).$client as
					| { transactionSync?: (fn: () => void) => void }
					| undefined;
				if (rawClient?.transactionSync) {
					await durableMigrate(client, config.migrations);
					return;
				}

				const migrationConfig = config.migrations as MigrationConfig;
				const statements = getMigrationStatements(migrationConfig);
				await client.execute("BEGIN");
				try {
					for (const statement of statements) {
						await client.execute(statement);
					}
					await client.execute("COMMIT");
				} catch (error) {
					await client.execute("ROLLBACK");
					throw error;
				}
			}
		},
		onDestroy: async (client) => {
			await client.close();
		},
	};
}
