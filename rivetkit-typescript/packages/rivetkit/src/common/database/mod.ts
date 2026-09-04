import { getLogger } from "@/common/log";
import type {
	DatabaseProvider,
	NativeDatabaseProvider,
	RawAccess,
	SqliteDatabase,
	SqliteExecuteResult,
	SqliteProfilingOptions,
	SqliteCommitMode,
	SqliteTransactionDatabase,
	SqliteTransactionOptions,
	SynchronousRawAccess,
	SynchronousTransactionHandle,
	SynchronousTransactionAccess,
} from "./config";
import {
	isManualTransactionControl,
	isSqliteBindingObject,
	MIGRATION_TRANSACTION_TIMEOUT_MS,
	normalizeSqliteBindings,
	runSqliteTransactionSync,
	toSqliteBindings,
	validateTransactionName,
	validateTransactionTimeout,
} from "./shared";

export type { RawAccess, SynchronousRawAccess } from "./config";

export interface DatabaseFactoryConfig {
	onMigrate?: (db: RawAccess) => Promise<void> | void;
	warnOnManualTransactions?: boolean;
	/**
	 * SQLite profiling configuration.
	 *
	 * @experimental This entire configuration surface is experimental and
	 * subject to change without notice.
	 */
	profiling?: SqliteProfilingOptions;
	/** Native SQLite commit durability mode. Defaults to `"awaited"`. */
	commitMode?: SqliteCommitMode;
}
const nativeStateTransactionOpeners = new WeakMap<
	NativeDatabaseProvider,
	(
		timeoutMs?: number,
		context?: unknown,
	) => Promise<SqliteTransactionDatabase>
>();
type NativeStateTransactionContext = {
	enter(): Promise<unknown>;
	exit(scope: unknown): void;
};
const nativeStateTransactionClientBinders = new WeakMap<
	object,
	(context: NativeStateTransactionContext) => object
>();

/** @internal */
export function registerNativeStateTransactionOpener<
	T extends NativeDatabaseProvider,
>(
	provider: T,
	opener: (
		timeoutMs?: number,
		context?: unknown,
	) => Promise<SqliteTransactionDatabase>,
): T {
	nativeStateTransactionOpeners.set(provider, opener);
	return provider;
}

/** @internal */
export function bindNativeStateTransactionContext<T>(
	client: T,
	context: NativeStateTransactionContext,
): T {
	if (
		(typeof client !== "object" || client === null) &&
		typeof client !== "function"
	) {
		return client;
	}
	const bind = nativeStateTransactionClientBinders.get(client as object);
	return (bind?.(context) ?? client) as T;
}

function hasMultipleStatements(query: string): boolean {
	const trimmed = query.trim().replace(/;+$/, "").trimEnd();
	return trimmed.includes(";");
}

function isTerminalTransactionError(error: unknown): boolean {
	if (typeof error !== "object" || error === null) return false;
	const code = (error as { code?: unknown }).code;
	return (
		code === "transaction_closed" ||
		code === "transaction_terminal" ||
		code === "transaction_expired"
	);
}

export function db({
	onMigrate,
	warnOnManualTransactions = true,
	profiling,
	commitMode = "awaited",
}: DatabaseFactoryConfig = {}): DatabaseProvider<SynchronousRawAccess> {
	const provider: DatabaseProvider<SynchronousRawAccess> = {
		sqliteProfiling: profiling,
		sqliteCommitMode: commitMode,
		createClient: async (ctx) => {
			const nativeDatabaseProvider = ctx.nativeDatabaseProvider;
			if (!nativeDatabaseProvider) {
				throw new Error(
					"native SQLite is required, but the current runtime did not provide a native database provider",
				);
			}

			const db = await nativeDatabaseProvider.open(ctx.actorId);
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
				if (typeof error !== "object" || error === null) return;
				const code = (error as { code?: unknown }).code;
				if (
					code === "transaction_active" ||
					code === "transaction_closed"
				) {
					closeSynchronousHandle?.();
				}
			};

			const createClient = (
				target: SqliteDatabase | SqliteTransactionDatabase,
				transactionScoped = false,
				stateTransactionContext?: NativeStateTransactionContext,
				sequencing: SqliteDatabase | SqliteTransactionDatabase = db,
			): SynchronousRawAccess => {
				const client: SynchronousRawAccess = {
					execute: async <
						TRow extends Record<string, unknown> = Record<
							string,
							unknown
						>,
					>(
						query: string,
						...args: unknown[]
					): Promise<TRow[]> => {
						ensureOpen();
						ensureSynchronousTransactionClient(
							transactionScoped,
							true,
						);
						if (
							!transactionScoped &&
							warnOnManualTransactions &&
							!manualTransactionWarned &&
							!hasMultipleStatements(query) &&
							isManualTransactionControl(query)
						) {
							manualTransactionWarned = true;
							getLogger("database").warn(
								{ actorId: ctx.actorId },
								"Manual cross-call SQLite transactions can interleave with other actor work. Use db.transaction() or db.transactionSync() for coordinated transactions. Set warnOnManualTransactions: false in your db(...) configuration to disable this warning.",
							);
						}

						const kvReadsBefore = ctx.metrics?.totalKvReads ?? 0;
						const kvWritesBefore = ctx.metrics?.totalKvWrites ?? 0;
						const start = performance.now();

						try {
							if (args.length > 0) {
								const bindings =
									args.length === 1 &&
									isSqliteBindingObject(args[0])
										? toSqliteBindings(args[0])
										: toSqliteBindings(args);
								const { rows, columns } = await target.execute(
									query,
									bindings,
								);
								return rows.map((row) =>
									rowToObject<TRow>(row, columns),
								);
							}

							if (!hasMultipleStatements(query)) {
								const { rows, columns } = await target.execute(
									query,
									undefined,
								);
								return rows.map((row) =>
									rowToObject<TRow>(row, columns),
								);
							}

							return await execMultiStatement<TRow>(
								target,
								query,
							);
						} finally {
							const durationMs = performance.now() - start;
							ctx.metrics?.trackSql(query, durationMs);
							if (ctx.metrics) {
								const kvReads =
									ctx.metrics.totalKvReads - kvReadsBefore;
								const kvWrites =
									ctx.metrics.totalKvWrites - kvWritesBefore;
								ctx.log?.debug({
									msg: "sql query",
									query: query.slice(0, 120),
									durationMs,
									kvReads,
									kvWrites,
								});
							}
						}
					},
					executeSync: <
						TRow extends Record<string, unknown> = Record<
							string,
							unknown
						>,
					>(
						query: string,
						...args: unknown[]
					): TRow[] => {
						ensureOpen();
						ensureSynchronousTransactionClient(transactionScoped);
						if (!target.executeSync) {
							throw new Error(
								"Synchronous SQLite queries are only available in the Node.js native runtime.",
							);
						}
						if (
							!transactionScoped &&
							warnOnManualTransactions &&
							!manualTransactionWarned &&
							!hasMultipleStatements(query) &&
							isManualTransactionControl(query)
						) {
							manualTransactionWarned = true;
							getLogger("database").warn(
								{ actorId: ctx.actorId },
								"Manual cross-call SQLite transactions can interleave with other actor work. Use db.transaction() or db.transactionSync() for coordinated transactions. Set warnOnManualTransactions: false in your db(...) configuration to disable this warning.",
							);
						}

						const kvReadsBefore = ctx.metrics?.totalKvReads ?? 0;
						const kvWritesBefore = ctx.metrics?.totalKvWrites ?? 0;
						const start = performance.now();

						try {
							if (args.length > 0) {
								const bindings =
									args.length === 1 &&
									isSqliteBindingObject(args[0])
										? toSqliteBindings(args[0])
										: toSqliteBindings(args);
								const { rows, columns } = target.executeSync(
									query,
									bindings,
								);
								return rows.map((row) =>
									rowToObject<TRow>(row, columns),
								);
							}

							if (!hasMultipleStatements(query)) {
								const { rows, columns } = target.executeSync(
									query,
									undefined,
								);
								return rows.map((row) =>
									rowToObject<TRow>(row, columns),
								);
							}

							return execMultiStatementSync<TRow>(target, query);
						} catch (error) {
							if (!transactionScoped)
								recoverSynchronousHandle(error);
							throw error;
						} finally {
							const durationMs = performance.now() - start;
							ctx.metrics?.trackSql(query, durationMs);
							if (ctx.metrics) {
								const kvReads =
									ctx.metrics.totalKvReads - kvReadsBefore;
								const kvWrites =
									ctx.metrics.totalKvWrites - kvWritesBefore;
								ctx.log?.debug({
									msg: "sql query",
									query: query.slice(0, 120),
									durationMs,
									kvReads,
									kvWrites,
								});
							}
						}
					},
					executeSyncRaw: (
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
							if (!transactionScoped)
								recoverSynchronousHandle(error);
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
					},
					transaction: async <T>(
						callback: (tx: RawAccess) => Promise<T> | T,
						options?: SqliteTransactionOptions,
					): Promise<T> => {
						ensureOpen();
						ensureSynchronousTransactionClient(
							transactionScoped,
							true,
						);
						validateTransactionTimeout(options?.timeout);
						validateTransactionName(options?.name);
						if (
							transactionScoped &&
							options?.experimental?.includeState
						) {
							throw new Error(
								"experimental.includeState is not supported for nested transactions",
							);
						}
						const includeState =
							options?.experimental?.includeState === true;
						const stateScope =
							includeState && stateTransactionContext
								? await stateTransactionContext.enter()
								: undefined;
						try {
							const transaction = includeState
								? await (() => {
										const beginStateTransaction =
											ctx.nativeDatabaseProvider &&
											nativeStateTransactionOpeners.get(
												ctx.nativeDatabaseProvider,
											);
										if (!beginStateTransaction) {
											throw new Error(
												"experimental.includeState is only supported by RivetKit's embedded database provider",
											);
										}
										return beginStateTransaction(
											options?.timeout,
											stateScope,
										);
									})()
								: await db.beginTransaction(
										options?.timeout,
										options?.name,
									);
							const tx = createClient(
								transaction,
								true,
								undefined,
								sequencing,
							);
							try {
								const result = await callback(tx);
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
						} finally {
							if (stateScope !== undefined) {
								stateTransactionContext?.exit(stateScope);
							}
						}
					},
					transactionSync: <T>(
						callback: (tx: SynchronousTransactionAccess) => T,
						options?: Omit<
							SqliteTransactionOptions,
							"experimental"
						>,
					): T => {
						ensureOpen();
						if (transactionScoped || synchronousTransactionKind) {
							throw new Error(
								"Nested synchronous SQLite transactions are not supported.",
							);
						}
						return runSqliteTransactionSync(
							db,
							(transaction) => {
								const transactionClient = createClient(
									transaction,
									true,
									undefined,
									sequencing,
								);
								const tx: SynchronousTransactionAccess = {
									executeSync: transactionClient.executeSync,
									commitSeq: transactionClient.commitSeq,
									flushedSeq: transactionClient.flushedSeq,
									waitForFlush:
										transactionClient.waitForFlush,
									flushError: transactionClient.flushError,
								};
								synchronousTransactionKind = "callback";
								try {
									return callback(tx);
								} finally {
									synchronousTransactionKind = undefined;
								}
							},
							options,
						);
					},
					commitSeq: () => sequencing.commitSeq!(),
					flushedSeq: () => sequencing.flushedSeq!(),
					waitForFlush: async (seq?: number) => {
						const targetSeq = seq ?? sequencing.commitSeq!();
						if (!Number.isSafeInteger(targetSeq) || targetSeq < 0) {
							throw new Error(
								"flush sequence must be a non-negative safe integer",
							);
						}
						await sequencing.waitForFlush!(targetSeq);
					},
					flushError: () => sequencing.flushError!(),
					beginTransactionSync: (
						options?: Omit<
							SqliteTransactionOptions,
							"experimental"
						>,
					): SynchronousTransactionHandle => {
						ensureOpen();
						if (transactionScoped || synchronousTransactionKind) {
							throw new Error(
								"Nested synchronous SQLite transactions are not supported.",
							);
						}
						validateTransactionTimeout(options?.timeout);
						validateTransactionName(options?.name);
						if (!db.beginTransactionSync) {
							throw new Error(
								"Synchronous SQLite transactions are only available in the Node.js native runtime.",
							);
						}
						const transaction = db.beginTransactionSync(
							options?.timeout,
							options?.name,
						);
						const transactionClient = createClient(
							transaction,
							true,
							undefined,
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
						const closeOnTerminal = (error: unknown) => {
							if (isTerminalTransactionError(error)) {
								finish();
							}
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
					},
					close: async () => {
						if (!closed) {
							closed = true;
							await db.close();
						}
					},
					nativeMetrics: () => db.nativeMetrics?.() ?? null,
				};
				if (!transactionScoped) {
					nativeStateTransactionClientBinders.set(client, (context) =>
						createClient(target, false, context, sequencing),
					);
				}
				return client;
			};
			const client = createClient(db);
			return client;
		},
		onMigrate: async (client) => {
			if (onMigrate) {
				await withMigrationSavepoint(client, (leased) =>
					onMigrate(leased),
				);
			}
		},
	};
	return provider;
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

async function execMultiStatement<TRow extends Record<string, unknown>>(
	db: Pick<SqliteDatabase, "exec">,
	query: string,
): Promise<TRow[]> {
	const results: Record<string, unknown>[] = [];
	let columnNames: string[] | null = null;
	await db.exec(query, (row: unknown[], columns: string[]) => {
		if (!columnNames) {
			columnNames = columns;
		}
		results.push(rowToObject(row, columnNames));
	});
	return results as TRow[];
}

function execMultiStatementSync<TRow extends Record<string, unknown>>(
	db: Pick<SqliteDatabase, "execSync">,
	query: string,
): TRow[] {
	if (!db.execSync) {
		throw new Error(
			"Synchronous SQLite queries are only available in the Node.js native runtime.",
		);
	}
	const results: Record<string, unknown>[] = [];
	let columnNames: string[] | null = null;
	db.execSync(query, (row: unknown[], columns: string[]) => {
		if (!columnNames) {
			columnNames = columns;
		}
		results.push(rowToObject(row, columnNames));
	});
	return results as TRow[];
}

async function withMigrationSavepoint<T>(
	client: RawAccess,
	callback: (leased: RawAccess) => Promise<T> | T,
): Promise<T> {
	return await client.transaction(
		async (transaction) => {
			await transaction.execute("SAVEPOINT __rivet_on_migrate");
			try {
				const result = await callback(transaction);
				await transaction.execute(
					"RELEASE SAVEPOINT __rivet_on_migrate",
				);
				return result;
			} catch (error) {
				try {
					await transaction.execute(
						"ROLLBACK TO SAVEPOINT __rivet_on_migrate",
					);
				} finally {
					await transaction.execute(
						"RELEASE SAVEPOINT __rivet_on_migrate",
					);
				}
				throw error;
			}
		},
		{
			name: "rivetkit-migration",
			timeout: MIGRATION_TRANSACTION_TIMEOUT_MS,
		},
	);
}
