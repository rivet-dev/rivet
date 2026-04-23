import { createHash } from "node:crypto";
import {
	drizzle,
	type RemoteCallback,
	type SqliteRemoteDatabase,
} from "drizzle-orm/sqlite-proxy";
import { AsyncMutex, toSqliteBindings } from "@/common/database/shared";
import type {
	DatabaseProvider,
	DatabaseProviderContext,
	RawAccess,
	SqliteDatabase,
} from "@/common/database/config";

export {
	alias,
	check,
	foreignKey,
	index,
	integer,
	primaryKey,
	sqliteTable,
	sqliteTableCreator,
	text,
	unique,
	uniqueIndex,
} from "drizzle-orm/sqlite-core";
export type { SQLiteTable } from "drizzle-orm/sqlite-core";

type DrizzleSchema = Record<string, unknown>;
type DrizzleDatabase<TSchema extends DrizzleSchema> =
	SqliteRemoteDatabase<TSchema> & RawAccess;

interface DrizzleMigrationJournalEntry {
	idx: number;
	tag: string;
	when: number;
	breakpoints?: boolean;
}

interface DrizzleMigrations {
	journal: unknown;
	migrations: Record<string, string>;
}

interface DrizzleDatabaseFactoryConfig<TSchema extends DrizzleSchema> {
	schema?: TSchema;
	migrations?: DrizzleMigrations;
	onMigrate?: (db: DrizzleDatabase<TSchema>) => Promise<void> | void;
}

interface DrizzleKitConfig {
	out?: string;
	schema?: string;
	dialect?: "sqlite";
	[key: string]: unknown;
}

export function defineConfig<TConfig extends DrizzleKitConfig>(
	config: TConfig,
): TConfig & { dialect: "sqlite" } {
	return {
		dialect: "sqlite",
		...config,
	};
}

export function db<TSchema extends DrizzleSchema = Record<string, never>>({
	schema,
	migrations,
	onMigrate,
}: DrizzleDatabaseFactoryConfig<TSchema> = {}): DatabaseProvider<
	DrizzleDatabase<TSchema>
> {
	return {
		createClient: async (ctx) => {
			const override = ctx.overrideDrizzleDatabaseClient
				? await ctx.overrideDrizzleDatabaseClient()
				: undefined;
			if (override) {
				return override as DrizzleDatabase<TSchema>;
			}

			const nativeDatabaseProvider = ctx.nativeDatabaseProvider;
			if (!nativeDatabaseProvider) {
				throw new Error(
					"native SQLite is required, but the current runtime did not provide a native database provider",
				);
			}

			const nativeDb = await nativeDatabaseProvider.open(ctx.actorId);
			ctx.metrics?.setSqliteVfsMetricsSource(() => {
				return nativeDb.getSqliteVfsMetrics?.() ?? null;
			});

			const mutex = new AsyncMutex();
			let closed = false;
			const ensureOpen = () => {
				if (closed) {
					throw new Error(
						"Database is closed. This usually means a background timer (setInterval, setTimeout) or a stray promise is still running after the actor stopped. Use c.abortSignal to clean up timers before the actor shuts down.",
					);
				}
			};

			const runSql = async (
				query: string,
				params: unknown[],
				method: "run" | "all" | "values" | "get",
			) => {
				return await mutex.run(async () => {
					ensureOpen();

					const start = performance.now();
					const kvReadsBefore = ctx.metrics?.totalKvReads ?? 0;
					const kvWritesBefore = ctx.metrics?.totalKvWrites ?? 0;
					try {
						if (method === "run") {
							await nativeDb.run(query, toSqliteBindings(params));
							return { rows: [] };
						}

						const { rows } = await nativeDb.query(
							query,
							toSqliteBindings(params),
						);
						if (method === "get") {
							return { rows: rows[0] };
						}
						return { rows };
					} finally {
						const durationMs = performance.now() - start;
						ctx.metrics?.trackSql(query, durationMs);
						if (ctx.metrics) {
							ctx.log?.debug({
								msg: "sql query",
								query: query.slice(0, 120),
								durationMs,
								kvReads: ctx.metrics.totalKvReads - kvReadsBefore,
								kvWrites:
									ctx.metrics.totalKvWrites - kvWritesBefore,
							});
						}
					}
				});
			};

			const callback: RemoteCallback = async (query, params, method) => {
				return await runSql(query, params, method);
			};

			const drizzleDb = drizzle(callback, {
				schema,
			}) as DrizzleDatabase<TSchema>;
			drizzleDb.execute = async <
				TRow extends Record<string, unknown> = Record<string, unknown>,
			>(
				query: string,
				...args: unknown[]
			): Promise<TRow[]> => {
				return await executeRaw<TRow>(
					nativeDb,
					mutex,
					ctx,
					ensureOpen,
					query,
					args,
				);
			};
			drizzleDb.close = async () => {
				const shouldClose = await mutex.run(async () => {
					if (closed) return false;
					closed = true;
					return true;
				});
				if (shouldClose) {
					await nativeDb.close();
				}
			};

			return drizzleDb;
		},
		onMigrate: async (client) => {
			if (migrations) {
				await runMigrations(client, migrations);
			}
			if (onMigrate) {
				await onMigrate(client);
			}
		},
		onDestroy: async (client) => {
			await client.close();
		},
	};
}

async function runMigrations<TSchema extends DrizzleSchema>(
	db: DrizzleDatabase<TSchema>,
	migrations: DrizzleMigrations,
) {
	const journal = parseMigrationJournal(migrations.journal);

	await db.execute(`
		CREATE TABLE IF NOT EXISTS __drizzle_migrations (
			id INTEGER PRIMARY KEY AUTOINCREMENT,
			hash TEXT NOT NULL,
			created_at NUMERIC
		)
	`);

	const rows = await db.execute<{ created_at: number }>(
		"SELECT created_at FROM __drizzle_migrations ORDER BY created_at DESC LIMIT 1",
	);
	const lastMigration = rows[0]?.created_at ?? 0;

	for (const entry of journal.entries) {
		if (lastMigration >= entry.when) {
			continue;
		}

		const key = `m${entry.idx.toString().padStart(4, "0")}`;
		const migration = migrations.migrations[key];
		if (migration === undefined) {
			throw new Error(
				`missing Drizzle migration "${key}" for journal entry "${entry.tag}"`,
			);
		}

		const statements = migration
			.split("--> statement-breakpoint")
			.map((statement) => statement.trim())
			.filter(Boolean);
		for (const statement of statements) {
			await db.execute(statement);
		}

		await db.execute(
			"INSERT INTO __drizzle_migrations (hash, created_at) VALUES (?, ?)",
			createHash("sha256").update(migration).digest("hex"),
			entry.when,
		);
	}
}

function parseMigrationJournal(journal: unknown): {
	entries: DrizzleMigrationJournalEntry[];
} {
	if (
		!journal ||
		typeof journal !== "object" ||
		!("entries" in journal) ||
		!Array.isArray(journal.entries)
	) {
		throw new Error("invalid Drizzle migration journal");
	}

	return journal as { entries: DrizzleMigrationJournalEntry[] };
}

function sqlReturnsRows(query: string): boolean {
	const token = query.trimStart().slice(0, 16).toUpperCase();
	if (token.startsWith("PRAGMA")) {
		return !/^PRAGMA\b[\s\S]*=/.test(query.trim());
	}
	return (
		token.startsWith("SELECT") ||
		token.startsWith("WITH") ||
		/\bRETURNING\b/i.test(query)
	);
}

function hasMultipleStatements(query: string): boolean {
	const trimmed = query.trim().replace(/;+$/, "").trimEnd();
	return trimmed.includes(";");
}

function rowToObject<TRow extends Record<string, unknown>>(
	row: unknown[],
	columns: string[],
): TRow {
	const rowObj: Record<string, unknown> = {};
	for (let i = 0; i < columns.length; i++) {
		rowObj[columns[i]] = row[i];
	}
	return rowObj as TRow;
}

async function executeRaw<TRow extends Record<string, unknown>>(
	db: SqliteDatabase,
	mutex: AsyncMutex,
	ctx: DatabaseProviderContext,
	ensureOpen: () => void,
	query: string,
	args: unknown[],
): Promise<TRow[]> {
	return await mutex.run(async () => {
		ensureOpen();

		const start = performance.now();
		const kvReadsBefore = ctx.metrics?.totalKvReads ?? 0;
		const kvWritesBefore = ctx.metrics?.totalKvWrites ?? 0;
		try {
			if (args.length > 0) {
				if (!sqlReturnsRows(query)) {
					await db.run(query, toSqliteBindings(args));
					return [];
				}

				const { rows, columns } = await db.query(
					query,
					toSqliteBindings(args),
				);
				return rows.map((row) => rowToObject<TRow>(row, columns));
			}

			if (!hasMultipleStatements(query)) {
				if (!sqlReturnsRows(query)) {
					await db.run(query);
					return [];
				}

				const { rows, columns } = await db.query(query);
				return rows.map((row) => rowToObject<TRow>(row, columns));
			}

			const results: Record<string, unknown>[] = [];
			let columnNames: string[] | null = null;
			await db.exec(query, (row, columns) => {
				if (!columnNames) {
					columnNames = columns;
				}
				results.push(rowToObject(row, columnNames));
			});
			return results as TRow[];
		} finally {
			const durationMs = performance.now() - start;
			ctx.metrics?.trackSql(query, durationMs);
			if (ctx.metrics) {
				ctx.log?.debug({
					msg: "sql query",
					query: query.slice(0, 120),
					durationMs,
					kvReads: ctx.metrics.totalKvReads - kvReadsBefore,
					kvWrites: ctx.metrics.totalKvWrites - kvWritesBefore,
				});
			}
		}
	});
}
