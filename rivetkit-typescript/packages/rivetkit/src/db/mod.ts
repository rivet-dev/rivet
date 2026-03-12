import type { DatabaseProvider, RawAccess } from "./config";
import {
	AsyncMutex,
	createActorKvStore,
	isSqliteBindingObject,
	toSqliteBindings,
} from "./shared";

export type { RawAccess } from "./config";

interface DatabaseFactoryConfig {
	onMigrate?: (db: RawAccess) => Promise<void> | void;
}

export function db({
	onMigrate,
}: DatabaseFactoryConfig = {}): DatabaseProvider<RawAccess> {
	const clientToKvStore = new WeakMap<
		object,
		ReturnType<typeof createActorKvStore>
	>();

	return {
		createClient: async (ctx) => {
			// Check if override is provided
			const override = ctx.overrideRawDatabaseClient
				? await ctx.overrideRawDatabaseClient()
				: undefined;

			if (override) {
				// Use the override
				return {
					execute: async <
						TRow extends Record<string, unknown> = Record<
							string,
							unknown
						>,
					>(
						query: string,
						...args: unknown[]
					): Promise<TRow[]> => {
						return await override.exec<TRow>(query, ...args);
					},
					close: async () => {
						// Override clients don't need cleanup
					},
				} satisfies RawAccess;
			}

			// Construct KV-backed client using actor driver's KV operations
			if (!ctx.sqliteVfs) {
				throw new Error(
					"SqliteVfs instance not provided in context. The driver must provide a sqliteVfs instance.",
				);
			}

			const kvStore = createActorKvStore(
				ctx.kv,
				ctx.metrics,
				ctx.preloadedEntries,
			);
			const db = await ctx.sqliteVfs.open(ctx.actorId, kvStore);
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
							const token = query
								.trimStart()
								.slice(0, 16)
								.toUpperCase();
							const returnsRows =
								token.startsWith("SELECT") ||
								token.startsWith("PRAGMA") ||
								token.startsWith("WITH") ||
								/\bRETURNING\b/i.test(query);

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
							const results: Record<string, unknown>[] = [];
							let columnNames: string[] | null = null;
							await db.exec(
								query,
								(row: unknown[], columns: string[]) => {
									if (!columnNames) {
										columnNames = columns;
									}
									const rowObj: Record<string, unknown> = {};
									for (let i = 0; i < row.length; i++) {
										rowObj[columnNames[i]] = row[i];
									}
									results.push(rowObj);
								},
							);
							result = results as TRow[];
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
			clientToKvStore.set(client, kvStore);
			return client;
		},
		onMigrate: async (client) => {
			// Clear preloaded entries before migrations run. Migrations may
			// write and re-read pages, and stale preload data would be
			// served instead of the freshly written values.
			clientToKvStore.get(client as object)?.clearPreload();
			if (onMigrate) {
				await onMigrate(client);
			}
		},
		onDestroy: async (client) => {
			await client.close();
		},
	};
}
