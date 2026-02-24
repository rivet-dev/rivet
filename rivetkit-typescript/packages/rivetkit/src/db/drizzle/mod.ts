import type { Database } from "@rivetkit/sqlite-vfs";
import {
	drizzle as proxyDrizzle,
	type SqliteRemoteDatabase,
} from "drizzle-orm/sqlite-proxy";
import type { DatabaseProvider, RawAccess } from "../config";
import { AsyncMutex, createActorKvStore, toSqliteBindings } from "../shared";

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
 * Create a sqlite-proxy async callback from a @rivetkit/sqlite Database
 */
function createProxyCallback(
	waDb: Database,
	mutex: AsyncMutex,
	isClosed: () => boolean,
) {
	return async (
		sql: string,
		params: any[],
		method: "run" | "all" | "values" | "get",
	): Promise<{ rows: any }> => {
		return mutex.run(async () => {
			if (isClosed()) {
				throw new Error("database is closed");
			}

			if (method === "run") {
				await waDb.run(sql, toSqliteBindings(params));
				return { rows: [] };
			}

			// For all/get/values, use parameterized query
			const result = await waDb.query(sql, toSqliteBindings(params));

			// drizzle's mapResultRow accesses rows by column index (positional arrays)
			// so we return raw arrays for all methods
			if (method === "get") {
				return { rows: result.rows[0] };
			}

			return { rows: result.rows };
		});
	};
}

/**
 * Run inline migrations via the @rivetkit/sqlite Database.
 * Migrations use the same embedded format as drizzle-orm's durable-sqlite.
 */
async function runInlineMigrations(
	waDb: Database,
	mutex: AsyncMutex,
	migrations: any,
): Promise<void> {
	// Create migrations table
	await mutex.run(() =>
		waDb.exec(`
		CREATE TABLE IF NOT EXISTS __drizzle_migrations (
			id INTEGER PRIMARY KEY AUTOINCREMENT,
			hash TEXT NOT NULL,
			created_at INTEGER
		)
	`),
	);

	// Get the last applied migration
	let lastCreatedAt = 0;
	await mutex.run(() =>
		waDb.exec(
			"SELECT id, hash, created_at FROM __drizzle_migrations ORDER BY created_at DESC LIMIT 1",
			(row) => {
				lastCreatedAt = Number(row[2]) || 0;
			},
		),
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
		await mutex.run(() => waDb.exec(sql));

		// Record migration
		await mutex.run(() =>
			waDb.run(
				"INSERT INTO __drizzle_migrations (hash, created_at) VALUES (?, ?)",
				[entry.tag, entry.when],
			),
		);
	}
}

export function db<
	TSchema extends Record<string, unknown> = Record<string, never>,
>(
	config?: DatabaseFactoryConfig<TSchema>,
): DatabaseProvider<SqliteRemoteDatabase<TSchema> & RawAccess> {
	// Store the @rivetkit/sqlite Database instance alongside the drizzle client
	let waDbInstance: Database | null = null;
	const mutex = new AsyncMutex();

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
			let closed = false;
			const ensureOpen = () => {
				if (closed) {
					throw new Error("database is closed");
				}
			};

			// Create the async proxy callback
			const callback = createProxyCallback(waDb, mutex, () => closed);

			// Create the drizzle instance using sqlite-proxy
			const client = proxyDrizzle<TSchema>(callback, config);

			return Object.assign(client, {
				execute: async <
					TRow extends Record<string, unknown> = Record<string, unknown>,
				>(
					query: string,
					...args: unknown[]
				): Promise<TRow[]> => {
					return mutex.run(async () => {
						ensureOpen();

						if (args.length > 0) {
							const result = await waDb.query(
								query,
								toSqliteBindings(args),
							);
							return result.rows.map((row: unknown[]) => {
								const obj: Record<string, unknown> = {};
								for (let i = 0; i < result.columns.length; i++) {
									obj[result.columns[i]] = row[i];
								}
								return obj;
							}) as TRow[];
						}
						// Use exec for non-parameterized queries since
						// @rivetkit/sqlite's query() can crash on some statements.
						const results: Record<string, unknown>[] = [];
						let columnNames: string[] | null = null;
						await waDb.exec(
							query,
							(row: unknown[], columns: string[]) => {
								if (!columnNames) {
									columnNames = columns;
								}
								const obj: Record<string, unknown> = {};
								for (let i = 0; i < row.length; i++) {
									obj[columnNames[i]] = row[i];
								}
								results.push(obj);
							},
						);
						return results as TRow[];
					});
				},
				close: async () => {
					await mutex.run(async () => {
						if (closed) {
							return;
						}
						closed = true;
						await waDb.close();
						waDbInstance = null;
					});
				},
			} satisfies RawAccess);
		},
		onMigrate: async (_client) => {
			if (config?.migrations && waDbInstance) {
				await runInlineMigrations(
					waDbInstance,
					mutex,
					config.migrations,
				);
			}
		},
		onDestroy: async (client) => {
			await client.close();
		},
	};
}
