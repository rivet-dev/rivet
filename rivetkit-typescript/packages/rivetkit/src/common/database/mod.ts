import type { DatabaseProvider, RawAccess, SqliteDatabase } from "./config";
import { isSqliteBindingObject, toSqliteBindings } from "./shared";

export type { RawAccess } from "./config";

interface DatabaseFactoryConfig {
	onMigrate?: (db: RawAccess) => Promise<void> | void;
}

type RawAccessWithWriteMode = RawAccess & {
	__rivetWriteMode: <T>(callback: () => Promise<T> | T) => Promise<T>;
};

function hasMultipleStatements(query: string): boolean {
	const trimmed = query.trim().replace(/;+$/, "").trimEnd();
	return trimmed.includes(";");
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
			let closed = false;
			const ensureOpen = () => {
				if (closed) {
					throw new Error(
						"Database is closed. This usually means a background timer (setInterval, setTimeout) or a stray promise is still running after the actor stopped. Use c.abortSignal to clean up timers before the actor shuts down.",
					);
				}
			};

			const client: RawAccessWithWriteMode = {
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
							const { rows, columns } = await db.execute(
								query,
								bindings,
							);
							return rows.map((row) =>
								rowToObject<TRow>(row, columns),
							);
						}

						if (!hasMultipleStatements(query)) {
							const { rows, columns } = await db.execute(query);
							return rows.map((row) =>
								rowToObject<TRow>(row, columns),
							);
						}

						return await execMultiStatement<TRow>(db, query);
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
				close: async () => {
					if (!closed) {
						closed = true;
						await db.close();
					}
				},
				__rivetWriteMode: async <T>(
					callback: () => Promise<T> | T,
				): Promise<T> => {
					return await db.writeMode(async () => await callback());
				},
			};
			return client;
		},
		onMigrate: async (client) => {
			if (onMigrate) {
				await dbWriteMode(client, () => onMigrate(client));
			}
		},
	};
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
	db: SqliteDatabase,
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

async function dbWriteMode<T>(
	client: RawAccess,
	callback: () => Promise<T> | T,
): Promise<T> {
	const maybeClient = client as RawAccess & {
		__rivetWriteMode?: <TInner>(
			callback: () => Promise<TInner> | TInner,
		) => Promise<TInner>;
	};
	if (maybeClient.__rivetWriteMode) {
		return await maybeClient.__rivetWriteMode(callback);
	}
	return await callback();
}
