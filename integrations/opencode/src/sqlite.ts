import { Sqlite } from "@opencode/core/database/sqlite";
import { Context, Effect, Exit, Layer, Option } from "effect";
import { Reactivity } from "effect/unstable/reactivity";
import { SqlClient, Statement } from "effect/unstable/sql";
import { classifySqliteError, SqlError } from "effect/unstable/sql/SqlError";
import type { RawAccess } from "rivetkit/db";

export type ActorSqlite = Pick<RawAccess, "execute" | "transaction">;
const Transaction = Context.Service<ActorSqlite>(
	"@rivet-dev/opencode/Transaction",
);

/** Uses Rivet's transaction handle, including for queries issued by Effect fibers. */
export function sqliteLayer(
	database: ActorSqlite,
): Layer.Layer<SqlClient.SqlClient> {
	const make = Effect.gen(function* () {
		const run = (query: string, params: readonly unknown[] = []) =>
			Effect.gen(function* () {
				const transaction = yield* Effect.serviceOption(Transaction);
				const db = Option.getOrElse(transaction, () => database);
				return yield* Effect.tryPromise({
					try: () => db.execute(query, ...params),
					catch: (cause) =>
						new SqlError({
							reason: classifySqliteError(cause, { operation: "execute" }),
						}),
				});
			});
		const connection = Sqlite.makeConnection(
			run,
			// Subqueries give duplicate SELECT column names unique SQLite aliases.
			(query, params) =>
				Effect.flatMap(
					run(
						/^\s*(select|with)\b/i.test(query)
							? `SELECT * FROM (${query})`
							: query,
						params,
					),
					(rows) =>
						Effect.sync(() =>
							rows.map((row) => {
								// RawAccess returns objects. JavaScript reorders numeric keys, so fail
								// rather than corrupt positional results if upstream adds such aliases.
								const keys = Object.keys(row);
								if (
									keys.length > 1 &&
									keys.some((key) => /^(0|[1-9]\d*)$/.test(key))
								)
									throw new Error(
										"OpenCode SQLite values require non-numeric column aliases",
									);
								return Object.values(row);
							}),
						),
				),
			{},
		);
		const client = yield* SqlClient.make({
			acquirer: Effect.succeed(connection),
			compiler: Statement.makeCompilerSqlite(),
			spanAttributes: [["db.system.name", "sqlite"]],
		});
		const withTransaction: SqlClient.SqlClient["withTransaction"] = <A, E, R>(
			operation: Effect.Effect<A, E, R>,
		) =>
			Effect.gen(function* () {
				const parent = yield* Effect.serviceOption(Transaction);
				if (Option.isSome(parent)) {
					return yield* Effect.die(
						new Error("Nested OpenCode SQLite transactions are unsupported"),
					);
				}
				const context = yield* Effect.context<R>();
				const exit = yield* Effect.tryPromise({
					try: (signal) =>
						database.transaction(async (tx) => {
							const result = await Effect.runPromiseExit(
								Effect.provideContext(
									operation,
									Context.add(
										Context.add(context, Transaction, tx),
										client.transactionService,
										[connection, 0],
									),
								),
								{ signal },
							);
							// Throwing forces Rivet to roll back; preserve the Effect cause below.
							if (Exit.isFailure(result)) throw result;
							return result;
						}),
					catch: (cause) => cause,
				}).pipe(
					Effect.catch((cause) => {
						if (Exit.isExit(cause)) return Effect.succeed(cause);
						return Effect.fail(
							new SqlError({
								reason: classifySqliteError(cause, {
									operation: "transaction",
								}),
							}),
						);
					}),
				);
				return yield* exit as Exit.Exit<A, E>;
			});
		return Object.assign(client, {
			withTransaction,
			transactionStatements: false,
		});
	});
	return Layer.effect(SqlClient.SqlClient, make).pipe(
		Layer.provide(Reactivity.layer),
	);
}
