import {
	drizzle,
	type RemoteCallback,
	type SqliteRemoteDatabase,
} from "drizzle-orm/sqlite-proxy";
import type {
	DatabaseProvider,
	DatabaseProviderContext,
	SqliteCommitMode,
	SqliteDatabase,
	SqliteExecuteResult,
	SqliteProfilingOptions,
	SqliteTransactionDatabase,
	SqliteTransactionOptions,
	SynchronousRawAccess,
	SynchronousTransactionAccess,
	SynchronousTransactionHandle,
} from "@/common/database/config";
import {
	isManualTransactionControl,
	MIGRATION_TRANSACTION_TIMEOUT_MS,
	normalizeSqliteBindings,
	runSqliteTransactionSync,
	toSqliteBindings,
	validateTransactionName,
	validateTransactionTimeout,
} from "@/common/database/shared";
import { getLogger } from "@/common/log";
import { sha256Hex } from "@/utils/crypto";

export type { SQLiteTable } from "drizzle-orm/sqlite-core";
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

type DrizzleSchema = Record<string, unknown>;
type DrizzleDatabase<TSchema extends DrizzleSchema> = Omit<
	SqliteRemoteDatabase<TSchema>,
	"transaction"
> &
	Omit<SynchronousRawAccess, "transaction"> & {
		transaction: <T>(
			callback: (tx: DrizzleDatabase<TSchema>) => Promise<T> | T,
			options?: SqliteTransactionOptions,
		) => Promise<T>;
	};

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

function isTerminalTransactionError(error: unknown): boolean {
	if (!error || typeof error !== "object") return false;
	const code =
		"code" in error ? (error as { code?: unknown }).code : undefined;
	return (
		code === "transaction_closed" ||
		code === "transaction_terminal" ||
		code === "transaction_expired"
	);
}

export interface DrizzleDatabaseFactoryConfig<TSchema extends DrizzleSchema> {
	schema?: TSchema;
	migrations?: DrizzleMigrations;
	onMigrate?: (db: DrizzleDatabase<TSchema>) => Promise<void> | void;
	warnOnManualTransactions?: boolean;
	profiling?: SqliteProfilingOptions;
	commitMode?: SqliteCommitMode;
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
	warnOnManualTransactions = true,
	profiling,
	commitMode = "awaited",
}: DrizzleDatabaseFactoryConfig<TSchema> = {}): DatabaseProvider<
	DrizzleDatabase<TSchema>
> {
	return {
		sqliteProfiling: profiling,
		sqliteCommitMode: commitMode,
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
			let closed = false;
			let manualTransactionWarned = false;
			let synchronousTransactionKind: "callback" | "handle" | undefined;
			let closeSynchronousHandle: (() => void) | undefined;
			const ensureOpen = () => {
				if (closed) {
					throw new Error(
						"Database is closed. This usually means a background timer (setInterval, setTimeout) or a stray promise is still running after the actor stopped. Use c.abortSignal to clean up timers before the actor shuts down.",
					);
				}
			};
			const ensureSynchronousTransactionClient = (
				transactionScoped: boolean,
				asyncMember = false,
			) => {
				if (
					!transactionScoped &&
					synchronousTransactionKind === "callback"
				) {
					throw new Error(
						"Use the transaction callback's tx value for queries inside db.transactionSync().",
					);
				}
				if (
					!transactionScoped &&
					!asyncMember &&
					synchronousTransactionKind === "handle"
				) {
					throw new Error(
						"Use the open synchronous transaction handle until it is committed or rolled back.",
					);
				}
			};
			const recoverSynchronousHandle = (error: unknown) => {
				if (!error || typeof error !== "object") return;
				const code =
					"code" in error
						? (error as { code?: unknown }).code
						: undefined;
				if (
					code === "transaction_active" ||
					code === "transaction_closed"
				) {
					closeSynchronousHandle?.();
				}
			};

			const createDrizzleClient = (
				target: SqliteDatabase | SqliteTransactionDatabase,
				transactionScoped = false,
				sequencing:
					| SqliteDatabase
					| SqliteTransactionDatabase = nativeDb,
			): DrizzleDatabase<TSchema> => {
				const runSql = async (
					query: string,
					params: unknown[],
					method: "run" | "all" | "values" | "get",
				) => {
					ensureOpen();
					ensureSynchronousTransactionClient(transactionScoped, true);
					warnForManualTransaction(query, transactionScoped);

					const start = performance.now();
					const kvReadsBefore = ctx.metrics?.totalKvReads ?? 0;
					const kvWritesBefore = ctx.metrics?.totalKvWrites ?? 0;
					try {
						const { rows } = await target.execute(
							query,
							toSqliteBindings(params),
						);
						if (method === "run") {
							return { rows: [] };
						}
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
								kvReads:
									ctx.metrics.totalKvReads - kvReadsBefore,
								kvWrites:
									ctx.metrics.totalKvWrites - kvWritesBefore,
							});
						}
					}
				};

				const callback: RemoteCallback = async (
					query,
					params,
					method,
				) => {
					return await runSql(query, params, method);
				};

				const drizzleDb = drizzle(callback, {
					schema,
				}) as unknown as DrizzleDatabase<TSchema>;
				drizzleDb.execute = async <
					TRow extends Record<string, unknown> = Record<
						string,
						unknown
					>,
				>(
					query: string,
					...args: unknown[]
				): Promise<TRow[]> => {
					ensureSynchronousTransactionClient(transactionScoped, true);
					return await executeRaw<TRow>(
						target,
						ctx,
						ensureOpen,
						query,
						args,
						() =>
							warnForManualTransaction(query, transactionScoped),
					);
				};
				drizzleDb.executeSync = <
					TRow extends Record<string, unknown> = Record<
						string,
						unknown
					>,
				>(
					query: string,
					...args: unknown[]
				): TRow[] => {
					ensureSynchronousTransactionClient(transactionScoped);
					try {
						return executeRawSync<TRow>(
							target,
							ctx,
							ensureOpen,
							query,
							args,
							() =>
								warnForManualTransaction(
									query,
									transactionScoped,
								),
						);
					} catch (error) {
						if (!transactionScoped) recoverSynchronousHandle(error);
						throw error;
					}
				};
				drizzleDb.transaction = async <T>(
					transactionCallback: (
						tx: DrizzleDatabase<TSchema>,
					) => Promise<T> | T,
					options?: SqliteTransactionOptions,
				): Promise<T> => {
					ensureOpen();
					ensureSynchronousTransactionClient(transactionScoped, true);
					validateTransactionTimeout(options?.timeout);
					validateTransactionName(options?.name);
					const transaction = await nativeDb.beginTransaction(
						options?.timeout,
						options?.name,
					);
					const tx = createDrizzleClient(
						transaction,
						true,
						sequencing,
					);
					try {
						const result = await transactionCallback(tx);
						await transaction.commit();
						return result;
					} catch (error) {
						try {
							await transaction.rollback();
						} catch {
							// Preserve the callback or commit error after expiry cleanup.
						}
						throw error;
					}
				};
				drizzleDb.transactionSync = <T>(
					transactionCallback: (
						tx: SynchronousTransactionAccess,
					) => T,
					options?: Omit<SqliteTransactionOptions, "experimental">,
				): T => {
					ensureOpen();
					if (transactionScoped || synchronousTransactionKind) {
						throw new Error(
							"Nested synchronous SQLite transactions are not supported.",
						);
					}
					return runSqliteTransactionSync(
						nativeDb,
						(transaction) => {
							const transactionClient = createDrizzleClient(
								transaction,
								true,
								sequencing,
							);
							const tx: SynchronousTransactionAccess = {
								executeSync: transactionClient.executeSync,
								commitSeq: transactionClient.commitSeq,
								flushedSeq: transactionClient.flushedSeq,
								waitForFlush: transactionClient.waitForFlush,
								flushError: transactionClient.flushError,
							};
							synchronousTransactionKind = "callback";
							try {
								return transactionCallback(tx);
							} finally {
								synchronousTransactionKind = undefined;
							}
						},
						options,
					);
				};
				drizzleDb.executeSyncRaw = (
					query: string,
					...args: unknown[]
				): SqliteExecuteResult & { readonly: boolean } => {
					ensureOpen();
					ensureSynchronousTransactionClient(transactionScoped);
					if (!target.executeSync) {
						throw new Error(
							"Synchronous SQLite queries are only available in the Node.js native runtime.",
						);
					}
					if (sequencing.supportsSyncMetadata?.() !== true) {
						throw new Error(
							"Synchronous SQLite metadata is only available for local native SQLite.",
						);
					}
					let result: SqliteExecuteResult;
					try {
						result = target.executeSync(
							query,
							normalizeSqliteBindings(args),
						);
					} catch (error) {
						if (!transactionScoped) recoverSynchronousHandle(error);
						throw error;
					}
					if (result.readonly === undefined) {
						throw new Error(
							"Synchronous SQLite metadata is only available in the Node.js native runtime.",
						);
					}
					return result as SqliteExecuteResult & {
						readonly: boolean;
					};
				};
				drizzleDb.commitSeq = () => sequencing.commitSeq!();
				drizzleDb.flushedSeq = () => sequencing.flushedSeq!();
				drizzleDb.waitForFlush = async (seq?: number) => {
					const targetSeq = seq ?? sequencing.commitSeq!();
					if (!Number.isSafeInteger(targetSeq) || targetSeq < 0) {
						throw new Error(
							"flush sequence must be a non-negative safe integer",
						);
					}
					await sequencing.waitForFlush!(targetSeq);
				};
				drizzleDb.flushError = () => sequencing.flushError!();
				drizzleDb.beginTransactionSync = (
					options?: Omit<SqliteTransactionOptions, "experimental">,
				): SynchronousTransactionHandle => {
					ensureOpen();
					if (transactionScoped || synchronousTransactionKind) {
						throw new Error(
							"Nested synchronous SQLite transactions are not supported.",
						);
					}
					validateTransactionTimeout(options?.timeout);
					validateTransactionName(options?.name);
					if (!nativeDb.beginTransactionSync) {
						throw new Error(
							"Synchronous SQLite transactions are only available in the Node.js native runtime.",
						);
					}
					const transaction = nativeDb.beginTransactionSync(
						options?.timeout,
						options?.name,
					);
					const transactionClient = createDrizzleClient(
						transaction,
						true,
						sequencing,
					);
					let isOpen = true;
					synchronousTransactionKind = "handle";
					const finish = () => {
						isOpen = false;
						if (closeSynchronousHandle === finish) {
							closeSynchronousHandle = undefined;
							synchronousTransactionKind = undefined;
						}
					};
					closeSynchronousHandle = finish;
					const closeOnTerminal = (error: unknown): never => {
						if (isTerminalTransactionError(error)) finish();
						throw error;
					};
					const requireOpen = () => {
						if (!isOpen)
							throw new Error(
								"SQLite transaction handle is closed.",
							);
					};
					return {
						executeSync: (query, ...args) => {
							requireOpen();
							try {
								return transactionClient.executeSync(
									query,
									...args,
								);
							} catch (error) {
								return closeOnTerminal(error);
							}
						},
						executeSyncRaw: (query, ...args) => {
							requireOpen();
							try {
								return transactionClient.executeSyncRaw(
									query,
									...args,
								);
							} catch (error) {
								return closeOnTerminal(error);
							}
						},
						execSync: (sql, callback) => {
							requireOpen();
							try {
								return transaction.execSync(sql, callback);
							} catch (error) {
								return closeOnTerminal(error);
							}
						},
						commitSync: () => {
							requireOpen();
							try {
								return transaction.commitSync();
							} finally {
								finish();
							}
						},
						rollbackSync: () => {
							requireOpen();
							try {
								transaction.rollbackSync();
							} finally {
								finish();
							}
						},
						get isOpen() {
							return isOpen;
						},
						commitSeq: transactionClient.commitSeq,
						flushedSeq: transactionClient.flushedSeq,
						waitForFlush: transactionClient.waitForFlush,
						flushError: transactionClient.flushError,
					};
				};
				drizzleDb.close = async () => {
					if (!closed) {
						closed = true;
						await nativeDb.close();
					}
				};

				return drizzleDb;
			};

			const warnForManualTransaction = (
				query: string,
				transactionScoped: boolean,
			) => {
				if (
					transactionScoped ||
					!warnOnManualTransactions ||
					manualTransactionWarned ||
					hasMultipleStatements(query) ||
					!isManualTransactionControl(query)
				) {
					return;
				}
				manualTransactionWarned = true;
				getLogger("database").warn(
					{ actorId: ctx.actorId },
					"Manual cross-call SQLite transactions can interleave with other actor work. Use db.transaction() or db.transactionSync() for coordinated transactions. Set warnOnManualTransactions: false in your db(...) configuration to disable this warning.",
				);
			};

			return createDrizzleClient(nativeDb);
		},
		onMigrate: async (client) => {
			if (!migrations && !onMigrate) {
				return;
			}
			await withMigrationSavepoint(client, async (leased) => {
				if (migrations) {
					await runMigrations(leased, migrations);
				}
				if (onMigrate) {
					await onMigrate(leased);
				}
			});
		},
	};
}

async function withMigrationSavepoint<TSchema extends DrizzleSchema, T>(
	client: DrizzleDatabase<TSchema>,
	callback: (leased: DrizzleDatabase<TSchema>) => Promise<T> | T,
): Promise<T> {
	return await client.transaction(
		async (leased) => {
			await leased.execute("SAVEPOINT __rivet_on_migrate");
			try {
				const result = await callback(leased);
				await leased.execute("RELEASE SAVEPOINT __rivet_on_migrate");
				return result;
			} catch (error) {
				try {
					await leased.execute(
						"ROLLBACK TO SAVEPOINT __rivet_on_migrate",
					);
				} finally {
					await leased.execute(
						"RELEASE SAVEPOINT __rivet_on_migrate",
					);
				}
				throw error;
			}
		},
		{
			name: "rivetkit-drizzle-migration",
			timeout: MIGRATION_TRANSACTION_TIMEOUT_MS,
		},
	);
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
			await sha256Hex(migration),
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
	db: SqliteDatabase | SqliteTransactionDatabase,
	ctx: DatabaseProviderContext,
	ensureOpen: () => void,
	query: string,
	args: unknown[],
	warnForManualTransaction: () => void,
): Promise<TRow[]> {
	ensureOpen();
	warnForManualTransaction();

	const start = performance.now();
	const kvReadsBefore = ctx.metrics?.totalKvReads ?? 0;
	const kvWritesBefore = ctx.metrics?.totalKvWrites ?? 0;
	try {
		if (args.length > 0) {
			const { rows, columns } = await db.execute(
				query,
				toSqliteBindings(args),
			);
			return rows.map((row) => rowToObject<TRow>(row, columns));
		}

		if (!hasMultipleStatements(query)) {
			const { rows, columns } = await db.execute(query, undefined);
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
}

function executeRawSync<TRow extends Record<string, unknown>>(
	db: SqliteDatabase | SqliteTransactionDatabase,
	ctx: DatabaseProviderContext,
	ensureOpen: () => void,
	query: string,
	args: unknown[],
	warnForManualTransaction: () => void,
): TRow[] {
	ensureOpen();
	warnForManualTransaction();
	if (!db.executeSync) {
		throw new Error(
			"Synchronous SQLite queries are only available in the Node.js native runtime.",
		);
	}

	const start = performance.now();
	const kvReadsBefore = ctx.metrics?.totalKvReads ?? 0;
	const kvWritesBefore = ctx.metrics?.totalKvWrites ?? 0;
	try {
		if (args.length > 0) {
			const { rows, columns } = db.executeSync(
				query,
				toSqliteBindings(args),
			);
			return rows.map((row) => rowToObject<TRow>(row, columns));
		}

		if (!hasMultipleStatements(query)) {
			const { rows, columns } = db.executeSync(query, undefined);
			return rows.map((row) => rowToObject<TRow>(row, columns));
		}

		if (!db.execSync) {
			throw new Error(
				"Synchronous SQLite queries are only available in the Node.js native runtime.",
			);
		}
		const results: Record<string, unknown>[] = [];
		let columnNames: string[] | null = null;
		db.execSync(query, (row, columns) => {
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
}
