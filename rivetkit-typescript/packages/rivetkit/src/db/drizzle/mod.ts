import {
	type BetterSQLite3Database,
	drizzle as sqliteDrizzle,
} from "drizzle-orm/better-sqlite3";
import {
	type SqliteRemoteDatabase,
	drizzle as proxyDrizzle,
} from "drizzle-orm/sqlite-proxy";
import type { KvVfsOptions } from "../sqlite-vfs";
import type { DatabaseProvider, RawAccess } from "../config";
import type { Database } from "@rivetkit/sqlite-vfs";

export * from "./sqlite-core";

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

/**
 * Create a sqlite-proxy async callback from a wa-sqlite Database
 */
function createProxyCallback(waDb: Database) {
	return async (
		sql: string,
		params: any[],
		method: "run" | "all" | "values" | "get",
	): Promise<{ rows: any }> => {
		if (method === "run") {
			await waDb.run(sql, params);
			return { rows: [] };
		}

		// For all/get/values, use parameterized query
		const result = await waDb.query(sql, params);

		// drizzle's mapResultRow accesses rows by column index (positional arrays)
		// so we return raw arrays for all methods
		if (method === "get") {
			return { rows: result.rows[0] };
		}

		return { rows: result.rows };
	};
}

/**
 * Run inline migrations via the wa-sqlite Database.
 * Migrations use the same embedded format as drizzle-orm's durable-sqlite.
 */
async function runInlineMigrations(
	waDb: Database,
	migrations: any,
): Promise<void> {
	// Create migrations table
	await waDb.exec(`
		CREATE TABLE IF NOT EXISTS __drizzle_migrations (
			id INTEGER PRIMARY KEY AUTOINCREMENT,
			hash TEXT NOT NULL,
			created_at INTEGER
		)
	`);

	// Get the last applied migration
	let lastCreatedAt = 0;
	await waDb.exec(
		"SELECT id, hash, created_at FROM __drizzle_migrations ORDER BY created_at DESC LIMIT 1",
		(row) => {
			lastCreatedAt = Number(row[2]) || 0;
		},
	);

	// Apply pending migrations from journal entries
	const journal = migrations.journal;
	if (!journal?.entries) return;

	for (const entry of journal.entries) {
		if (entry.when <= lastCreatedAt) continue;

		// Find the migration SQL from the migrations map
		// The key format is "m" + zero-padded index (e.g. "m0000")
		const migrationKey = `m${String(entry.idx).padStart(4, "0")}`;
		const sql = migrations.migrations[migrationKey];
		if (!sql) continue;

		// Execute migration SQL
		await waDb.exec(sql);

		// Record migration
		await waDb.exec(
			`INSERT INTO __drizzle_migrations (hash, created_at) VALUES ('${entry.tag}', ${entry.when})`,
		);
	}
}

export function db<
	TSchema extends Record<string, unknown> = Record<string, never>,
>(
	config?: DatabaseFactoryConfig<TSchema>,
): DatabaseProvider<SqliteRemoteDatabase<TSchema> & RawAccess> {
	// Store the wa-sqlite Database instance alongside the drizzle client
	let waDbInstance: Database | null = null;

	return {
		createClient: async (ctx) => {
			// Construct KV-backed client using actor driver's KV operations
			if (!ctx.sqliteVfs) {
				throw new Error(
					"SqliteVfs instance not provided in context. The driver must provide a sqliteVfs instance.",
				);
			}

			const kvStore = createActorKvStore(ctx.kv);
			const waDb = await ctx.sqliteVfs.open(ctx.actorId, kvStore);
			waDbInstance = waDb;

			// Create the async proxy callback
			const callback = createProxyCallback(waDb);

			// Create the drizzle instance using sqlite-proxy
			const client = proxyDrizzle<TSchema>(callback, config);

			return Object.assign(client, {
				execute: async <
					TRow extends Record<string, unknown> = Record<string, unknown>,
				>(
					query: string,
					...args: unknown[]
				): Promise<TRow[]> => {
					if (args.length > 0) {
						const { rows, columns } = await waDb.query(query, args);
						return rows.map((row: unknown[]) => {
							const rowObj: Record<string, unknown> = {};
							for (let i = 0; i < row.length; i++) {
								rowObj[columns[i]] = row[i];
							}
							return rowObj;
						}) as TRow[];
					}

					const results: Record<string, unknown>[] = [];
					let columnNames: string[] | null = null;
					await waDb.exec(query, (row: unknown[], columns: string[]) => {
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
				},
				close: async () => {
					await waDb.close();
					waDbInstance = null;
				},
			} satisfies RawAccess);
		},
		onMigrate: async (_client) => {
			if (config?.migrations && waDbInstance) {
				await runInlineMigrations(waDbInstance, config.migrations);
			}
		},
		onDestroy: async (client) => {
			await client.close();
		},
	};
}
