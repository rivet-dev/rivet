import type { DatabaseProvider, RawAccess } from "./config";
import { AsyncMutex, isSqliteBindingObject, toSqliteBindings } from "./shared";

export type { RawAccess } from "./config";

interface DatabaseFactoryConfig {
	onMigrate?: (db: RawAccess) => Promise<void> | void;
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

function isPragmaAssignment(query: string): boolean {
	return /^PRAGMA\b[\s\S]*=/.test(query.trim());
}

export function db({
	onMigrate,
}: DatabaseFactoryConfig = {}): DatabaseProvider<RawAccess> {
	return {
		createClient: async (ctx) => {
			const nativeDatabaseProvider = ctx.nativeDatabaseProvider;
			if (!nativeDatabaseProvider) {
				throw new Error(
					"native SQLite is required, but the current runtime did not provide a native database provider",
				);
			}

			const db = await nativeDatabaseProvider.open(ctx.actorId);
			ctx.metrics?.setSqliteVfsMetricsSource(() => {
				return db.getSqliteVfsMetrics?.() ?? null;
			});
			let closed = false;
			const mutex = new AsyncMutex();
			const ensureOpen = () => {
				if (closed) {
					throw new Error(
						"Database is closed. This usually means a background timer (setInterval, setTimeout) or a stray promise is still running after the actor stopped. Use c.abortSignal to clean up timers before the actor shuts down.",
					);
				}
			};

			const client = {
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

						// `db.exec` does not support binding `?` placeholders.
						// Use `db.query` for statements that return rows and `db.run` for
						// statements that mutate data when parameters are provided.
						// Keep using `db.exec` for non-parameterized SQL because it
						// supports multi-statement migrations.
						let result: TRow[];
						if (args.length > 0) {
							const bindings =
								args.length === 1 &&
								isSqliteBindingObject(args[0])
									? toSqliteBindings(args[0])
									: toSqliteBindings(args);
							const returnsRows = sqlReturnsRows(query);

							if (returnsRows) {
								const { rows, columns } = await db.query(
									query,
									bindings,
								);
								result = rows.map((row: unknown[]) => {
									const rowObj: Record<string, unknown> = {};
									for (let i = 0; i < columns.length; i++) {
										rowObj[columns[i]] = row[i];
									}
									return rowObj;
								}) as TRow[];
							} else {
								await db.run(query, bindings);
								result = [] as TRow[];
							}
						} else {
							const returnsRows = sqlReturnsRows(query);
							if (!hasMultipleStatements(query)) {
								if (returnsRows) {
									const { rows, columns } =
										await db.query(query);
									result = rows.map((row: unknown[]) => {
										const rowObj: Record<string, unknown> =
											{};
										for (
											let i = 0;
											i < columns.length;
											i++
										) {
											rowObj[columns[i]] = row[i];
										}
										return rowObj;
									}) as TRow[];
								} else if (isPragmaAssignment(query)) {
									await db.run(query);
									result = [] as TRow[];
								} else {
									const results: Record<string, unknown>[] =
										[];
									let columnNames: string[] | null = null;
									await db.exec(
										query,
										(row: unknown[], columns: string[]) => {
											if (!columnNames) {
												columnNames = columns;
											}
											const rowObj: Record<
												string,
												unknown
											> = {};
											for (
												let i = 0;
												i < row.length;
												i++
											) {
												rowObj[columnNames[i]] = row[i];
											}
											results.push(rowObj);
										},
									);
									result = results as TRow[];
								}
							} else {
								const results: Record<string, unknown>[] = [];
								let columnNames: string[] | null = null;
								await db.exec(
									query,
									(row: unknown[], columns: string[]) => {
										if (!columnNames) {
											columnNames = columns;
										}
										const rowObj: Record<string, unknown> =
											{};
										for (let i = 0; i < row.length; i++) {
											rowObj[columnNames[i]] = row[i];
										}
										results.push(rowObj);
									},
								);
								result = results as TRow[];
							}
						}

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
						return result;
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
			} satisfies RawAccess;
			return client;
		},
		onMigrate: async (client) => {
			if (onMigrate) {
				await onMigrate(client);
			}
		},
	};
}
