import type { DatabaseProvider, RawAccess } from "./config";
import { AsyncMutex, createActorKvStore, toSqliteBindings } from "./shared";

export type { RawAccess } from "./config";

interface DatabaseFactoryConfig {
	onMigrate?: (db: RawAccess) => Promise<void> | void;
}

export function db({
	onMigrate,
}: DatabaseFactoryConfig = {}): DatabaseProvider<RawAccess> {
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
						TRow extends Record<string, unknown> = Record<string, unknown>,
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
				throw new Error("SqliteVfs instance not provided in context. The driver must provide a sqliteVfs instance.");
			}

			const kvStore = createActorKvStore(ctx.kv);
			const db = await ctx.sqliteVfs.open(ctx.actorId, kvStore);
			let closed = false;
			const mutex = new AsyncMutex();
			const ensureOpen = () => {
				if (closed) {
					throw new Error("database is closed");
				}
			};

			return {
				execute: async <
					TRow extends Record<string, unknown> = Record<string, unknown>,
				>(
					query: string,
					...args: unknown[]
				): Promise<TRow[]> => {
					return await mutex.run(async () => {
						ensureOpen();

						// `db.exec` does not support binding `?` placeholders.
						// Use `db.query` for statements that return rows and `db.run` for
						// statements that mutate data when parameters are provided.
						// Keep using `db.exec` for non-parameterized SQL because it
						// supports multi-statement migrations.
						if (args.length > 0) {
							const bindings = toSqliteBindings(args);
							const token = query.trimStart().slice(0, 16).toUpperCase();
							const returnsRows =
								token.startsWith("SELECT") ||
								token.startsWith("PRAGMA") ||
								token.startsWith("WITH");

							if (returnsRows) {
								const { rows, columns } = await db.query(query, bindings);
								return rows.map((row: unknown[]) => {
									const rowObj: Record<string, unknown> = {};
									for (let i = 0; i < columns.length; i++) {
										rowObj[columns[i]] = row[i];
									}
									return rowObj;
								}) as TRow[];
							}

							await db.run(query, bindings);
							return [] as TRow[];
						}

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
						return results as TRow[];
					});
				},
				close: async () => {
					await mutex.run(async () => {
						if (closed) {
							return;
						}
						closed = true;
						await db.close();
					});
				},
			} satisfies RawAccess;
		},
		onMigrate: async (client) => {
			if (onMigrate) {
				await onMigrate(client);
			}
		},
		onDestroy: async (client) => {
			await client.close();
		},
	};
}
