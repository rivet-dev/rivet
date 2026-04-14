import { createRequire } from "node:module";
import {
	drizzle as proxyDrizzle,
	type SqliteRemoteDatabase,
} from "drizzle-orm/sqlite-proxy";
import type {
	DatabaseProvider,
	RawAccess,
	RawDatabaseClient,
	SqliteDatabase,
} from "../config";
import {
	AsyncMutex,
	isSqliteBindingObject,
	toSqliteBindings,
} from "../shared";

export * from "./sqlite-core";

import { type Config, defineConfig as originalDefineConfig } from "drizzle-kit";

/**
 * Supported drizzle-orm version bounds. Update these when testing confirms
 * compatibility with new releases. Run scripts/test-drizzle-compat.sh to
 * validate.
 */
const DRIZZLE_MIN = [0, 44, 0];
const DRIZZLE_MAX = [0, 46, 0]; // exclusive

let drizzleVersionChecked = false;

function compareVersions(a: number[], b: number[]): number {
	for (let i = 0; i < Math.max(a.length, b.length); i++) {
		const diff = (a[i] ?? 0) - (b[i] ?? 0);
		if (diff !== 0) return diff;
	}
	return 0;
}

function isSupported(version: string): boolean {
	// Strip prerelease suffix (e.g. "0.45.1-a7a15d0" -> "0.45.1")
	const v = version.replace(/-.*$/, "").split(".").map(Number);
	return (
		compareVersions(v, DRIZZLE_MIN) >= 0 &&
		compareVersions(v, DRIZZLE_MAX) < 0
	);
}

function checkDrizzleVersion() {
	if (drizzleVersionChecked) return;
	drizzleVersionChecked = true;

	try {
		const require = createRequire(import.meta.url);
		const { version } = require("drizzle-orm/package.json") as {
			version: string;
		};
		if (!isSupported(version)) {
			console.warn(
				`[rivetkit] drizzle-orm@${version} has not been tested with this version of rivetkit. ` +
					`Supported: >= ${DRIZZLE_MIN.join(".")} and < ${DRIZZLE_MAX.join(".")}. ` +
					`Things may still work, but please report issues at https://github.com/rivet-dev/rivet/issues`,
			);
		}
	} catch {
		// Cannot determine version, skip check.
	}
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
 * Create a sqlite-proxy async callback from a native SQLite database handle.
 */
function createProxyCallback(
	db: SqliteDatabase,
	mutex: AsyncMutex,
	isClosed: () => boolean,
	metrics?: import("@/actor/metrics").ActorMetrics,
	log?: { debug(obj: Record<string, unknown>): void },
) {
	return async (
		sql: string,
		params: any[],
		method: "run" | "all" | "values" | "get",
	): Promise<{ rows: any }> => {
		return await mutex.run(async () => {
			if (isClosed()) {
				throw new Error(
					"Database is closed. This usually means a background timer (setInterval, setTimeout) or a stray promise is still running after the actor stopped. Use c.abortSignal to clean up timers before the actor shuts down.",
				);
			}

			const kvReadsBefore = metrics?.totalKvReads ?? 0;
			const kvWritesBefore = metrics?.totalKvWrites ?? 0;
			const start = performance.now();

			let result: { rows: any };
			if (method === "run") {
				await db.run(sql, toSqliteBindings(params));
				result = { rows: [] };
			} else {
				const queryResult = await db.query(sql, toSqliteBindings(params));

				// drizzle's mapResultRow accesses rows by column index (positional arrays)
				// so we return raw arrays for all methods
				if (method === "get") {
					result = { rows: queryResult.rows[0] };
				} else {
					result = { rows: queryResult.rows };
				}
			}

			const durationMs = performance.now() - start;
			metrics?.trackSql(sql, durationMs);
			if (metrics && log) {
				const kvReads = metrics.totalKvReads - kvReadsBefore;
				const kvWrites = metrics.totalKvWrites - kvWritesBefore;
				log.debug({
					msg: "sql query",
					query: sql.slice(0, 120),
					durationMs,
					kvReads,
					kvWrites,
				});
			}
			return result;
		});
	};
}

function createProxyCallbackFromRawExecutor(
	rawDb: RawDatabaseClient,
	mutex: AsyncMutex,
	isClosed: () => boolean,
	metrics?: import("@/actor/metrics").ActorMetrics,
	log?: { debug(obj: Record<string, unknown>): void },
) {
	return async (
		sql: string,
		params: any[],
		method: "run" | "all" | "values" | "get",
	): Promise<{ rows: any }> => {
		return await mutex.run(async () => {
			if (isClosed()) {
				throw new Error(
					"Database is closed. This usually means a background timer (setInterval, setTimeout) or a stray promise is still running after the actor stopped. Use c.abortSignal to clean up timers before the actor shuts down.",
				);
			}

			const kvReadsBefore = metrics?.totalKvReads ?? 0;
			const kvWritesBefore = metrics?.totalKvWrites ?? 0;
			const start = performance.now();

			const rows = await rawDb.exec<Record<string, unknown>>(
				sql,
				...params,
			);
			const positionalRows = rows.map((row) => Object.values(row));

			const durationMs = performance.now() - start;
			metrics?.trackSql(sql, durationMs);
			if (metrics && log) {
				const kvReads = metrics.totalKvReads - kvReadsBefore;
				const kvWrites = metrics.totalKvWrites - kvWritesBefore;
				log.debug({
					msg: "sql query",
					query: sql.slice(0, 120),
					durationMs,
					kvReads,
					kvWrites,
				});
			}

			if (method === "run") {
				return { rows: [] };
			}

			if (method === "get") {
				return { rows: positionalRows[0] };
			}

			return { rows: positionalRows };
		});
	};
}

/**
 * Run inline migrations via the native SQLite database handle.
 */
async function runInlineMigrations(
	db: SqliteDatabase,
	migrations: any,
): Promise<void> {
	await db.exec(`
		CREATE TABLE IF NOT EXISTS __drizzle_migrations (
			id INTEGER PRIMARY KEY AUTOINCREMENT,
			hash TEXT NOT NULL,
			created_at INTEGER
		)
	`);

	// Get the last applied migration
	let lastCreatedAt = 0;
	await db.exec(
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

		await db.exec(sql);

		await db.run(
			"INSERT INTO __drizzle_migrations (hash, created_at) VALUES (?, ?)",
			[entry.tag, entry.when],
		);
	}
}

async function runInlineMigrationsWithRawExecutor(
	rawDb: RawDatabaseClient,
	migrations: any,
): Promise<void> {
	await rawDb.exec(`
		CREATE TABLE IF NOT EXISTS __drizzle_migrations (
			id INTEGER PRIMARY KEY AUTOINCREMENT,
			hash TEXT NOT NULL,
			created_at INTEGER
		)
	`);

	const lastRows = await rawDb.exec<{
		id: number;
		hash: string;
		created_at: number | null;
	}>(
		"SELECT id, hash, created_at FROM __drizzle_migrations ORDER BY created_at DESC LIMIT 1",
	);
	const lastCreatedAt = Number(lastRows[0]?.created_at ?? 0) || 0;

	const journal = migrations.journal;
	if (!journal?.entries) return;

	for (const entry of journal.entries) {
		if (entry.when <= lastCreatedAt) continue;

		const migrationKey = `m${String(entry.idx).padStart(4, "0")}`;
		const sql = migrations.migrations[migrationKey];
		if (!sql) continue;

		await rawDb.exec(sql);
		await rawDb.exec(
			"INSERT INTO __drizzle_migrations (hash, created_at) VALUES (?, ?)",
			entry.tag,
			entry.when,
		);
	}
}

export function db<
	TSchema extends Record<string, unknown> = Record<string, never>,
>(
	config?: DatabaseFactoryConfig<TSchema>,
): DatabaseProvider<SqliteRemoteDatabase<TSchema> & RawAccess> {
	checkDrizzleVersion();

	const clientToRawDb = new WeakMap<object, SqliteDatabase>();
	const clientToRawExecutor = new WeakMap<object, RawDatabaseClient>();

	return {
		createClient: async (ctx) => {
			const drizzleOverride = ctx.overrideDrizzleDatabaseClient
				? await ctx.overrideDrizzleDatabaseClient()
				: undefined;
			if (drizzleOverride) {
				return drizzleOverride as SqliteRemoteDatabase<TSchema> &
					RawAccess;
			}

			const rawOverride = ctx.overrideRawDatabaseClient
				? await ctx.overrideRawDatabaseClient()
				: undefined;
			if (rawOverride) {
				const mutex = new AsyncMutex();
				let closed = false;
				const callback = createProxyCallbackFromRawExecutor(
					rawOverride,
					mutex,
					() => closed,
					ctx.metrics,
					ctx.log,
				);
				const client = proxyDrizzle<TSchema>(callback, config);
				const result = Object.assign(client, {
					execute: async <
						TRow extends Record<string, unknown> = Record<
							string,
							unknown
						>,
					>(
						query: string,
						...args: unknown[]
					): Promise<TRow[]> => {
						return await rawOverride.exec<TRow>(query, ...args);
					},
					close: async () => {
						closed = true;
					},
				} satisfies RawAccess);
				clientToRawExecutor.set(result, rawOverride);
				return result;
			}

			if (!ctx.nativeDatabaseProvider) {
				throw new Error(
					"native SQLite is required, but the current runtime did not provide a native database provider",
				);
			}

			const db = await ctx.nativeDatabaseProvider.open(ctx.actorId);
			const mutex = new AsyncMutex();
			let closed = false;
			const ensureOpen = () => {
				if (closed) {
					throw new Error(
						"Database is closed. This usually means a background timer (setInterval, setTimeout) or a stray promise is still running after the actor stopped. Use c.abortSignal to clean up timers before the actor shuts down.",
					);
				}
			};

			// Create the async proxy callback
			const callback = createProxyCallback(
				db,
				mutex,
				() => closed,
				ctx.metrics,
				ctx.log,
			);

			// Create the drizzle instance using sqlite-proxy
			const client = proxyDrizzle<TSchema>(callback, config);

			const result = Object.assign(client, {
				execute: async <
					TRow extends Record<string, unknown> = Record<
						string,
						unknown
					>,
				>(
					query: string,
					...args: unknown[]
				): Promise<TRow[]> => {
					return await mutex.run(async () => {
						ensureOpen();

						const kvReadsBefore = ctx.metrics?.totalKvReads ?? 0;
						const kvWritesBefore = ctx.metrics?.totalKvWrites ?? 0;
						const start = performance.now();
						let rows: TRow[];

						if (args.length > 0) {
							const bindings =
								args.length === 1 &&
								isSqliteBindingObject(args[0])
									? toSqliteBindings(args[0])
									: toSqliteBindings(args);
							const result = await db.query(
								query,
								bindings,
							);
							rows = result.rows.map((row: unknown[]) => {
								const obj: Record<string, unknown> = {};
								for (
									let i = 0;
									i < result.columns.length;
									i++
								) {
									obj[result.columns[i]] = row[i];
								}
								return obj;
							}) as TRow[];
						} else {
							const results: Record<string, unknown>[] = [];
							let columnNames: string[] | null = null;
							await db.exec(
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
							rows = results as TRow[];
						}

						const durationMs = performance.now() - start;
						if (ctx.metrics && ctx.log) {
							const kvReads =
								ctx.metrics.totalKvReads - kvReadsBefore;
							const kvWrites =
								ctx.metrics.totalKvWrites - kvWritesBefore;
							ctx.log.debug({
								msg: "sql query",
								query: query.slice(0, 120),
								durationMs,
								kvReads,
								kvWrites,
							});
						}
						return rows;
					});
				},
				close: async () => {
					const shouldClose = await mutex.run(async () => {
						if (closed) return false;
						closed = true;
						return true;
					});
					if (shouldClose) {
						await db.close();
					}
				},
			} satisfies RawAccess);

			clientToRawDb.set(result, db);
			return result;
		},
		onMigrate: async (client) => {
			const db = clientToRawDb.get(client as object);
			if (config?.migrations && db) {
				await runInlineMigrations(db, config.migrations);
				return;
			}
			const rawExecutor = clientToRawExecutor.get(client as object);
			if (config?.migrations && rawExecutor) {
				await runInlineMigrationsWithRawExecutor(
					rawExecutor,
					config.migrations,
				);
			}
		},
		onDestroy: async (client) => {
			await client.close();
		},
	};
}
