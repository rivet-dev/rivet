import { Data, Effect, Stream } from "effect";
import { Reactivity } from "effect/reactivity";
import { SqlClient, type SqlConnection, SqlError, Statement } from "effect/sql";
import type { RawAccess } from "rivetkit/db";

type RivetkitDatabase = Pick<RawAccess, "execute" | "transaction">;

const spanAttributes = [["db.system.name", "sqlite"]] as const;

const toSqlError = (cause: unknown, message: string, operation: string) =>
	new SqlError.SqlError({
		reason: SqlError.classifySqliteError(cause, { message, operation }),
	});

const makeConnection = (
	database: Pick<RawAccess, "execute">,
): SqlConnection.Connection => {
	const run = (sql: string, params: ReadonlyArray<unknown>) =>
		Effect.tryPromise({
			try: () => database.execute(sql, ...params),
			catch: (cause) =>
				toSqlError(cause, "Failed to execute statement", "execute"),
		});
	// RivetKit returns rows as objects whose keys follow column order.
	const runValues = (sql: string, params: ReadonlyArray<unknown>) =>
		Effect.map(run(sql, params), (rows) =>
			rows.map((row) => Object.values(row)),
		);
	const execute: SqlConnection.Connection["execute"] = (
		sql,
		params,
		transformRows,
	) =>
		transformRows
			? Effect.map(run(sql, params), transformRows)
			: run(sql, params);

	return {
		execute,
		executeRaw: run,
		executeValues: runValues,
		executeValuesUnprepared: runValues,
		executeUnprepared: execute,
		executeStream: (sql, params, transformRows) =>
			Stream.fromIterableEffect(execute(sql, params, transformRows)),
	};
};

// Rejects the parked `db.transaction` callback so RivetKit rolls back.
const rollbackSignal = Symbol("@rivetkit/effect/SqlClient/rollback");

/**
 * The transaction control statements `SqlClient.make` sends. The transaction
 * connection never runs them as SQL: RivetKit's `db.transaction` owns the
 * transaction, so the commit statement resolves its callback and the others
 * do nothing.
 */
const beginStatement = "@rivetkit/effect/SqlClient/begin";
const commitStatement = "@rivetkit/effect/SqlClient/commit";
const rollbackStatement = "@rivetkit/effect/SqlClient/rollback";

/**
 * Effect 4.0 turns a commit failure into a defect and later versions keep it
 * a failure, so it always crosses `withTransaction` as this defect.
 */
class CommitFailure extends Data.Error<{
	readonly error: SqlError.SqlError;
}> {}

interface RivetkitTransaction {
	readonly connection: SqlConnection.Connection;
	readonly rollbackUnlessCommitted: Effect.Effect<void>;
}

/**
 * Opens a RivetKit `db.transaction` and parks its callback, so the
 * transaction body runs on the calling fiber. The commit statement commits;
 * closing the scope without one rolls back.
 */
const beginRivetkitTransaction = (database: RivetkitDatabase) =>
	Effect.acquireRelease(
		Effect.callback<RivetkitTransaction, SqlError.SqlError>((resume) => {
			const done: Promise<void> = database
				.transaction(
					(tx) =>
						new Promise<void>((resolve, reject) => {
							let commitStarted = false;
							const end = (commit: boolean) =>
								Effect.tryPromise({
									try: () => {
										if (commit) resolve();
										else reject(rollbackSignal);
										return done;
									},
									catch: (cause) =>
										toSqlError(
											cause,
											"Failed to end transaction",
											"transaction",
										),
								});
							const commit = Effect.suspend(() => {
								commitStarted = true;
								return end(true);
							}).pipe(
								Effect.mapError(
									(error) => new CommitFailure({ error }),
								),
								Effect.orDie,
								Effect.as([]),
							);
							const connection = makeConnection(tx);
							resume(
								Effect.succeed({
									connection: {
										...connection,
										executeUnprepared: (
											sql,
											params,
											transformRows,
										) => {
											switch (sql) {
												case beginStatement:
												case rollbackStatement:
													return Effect.succeed([]);
												case commitStatement:
													return commit;
												default:
													return connection.executeUnprepared(
														sql,
														params,
														transformRows,
													);
											}
										},
									},
									rollbackUnlessCommitted: Effect.suspend(
										() =>
											commitStarted
												? Effect.void
												: Effect.orDie(end(false)),
									),
								}),
							);
						}),
				)
				.catch((cause: unknown) => {
					if (cause !== rollbackSignal) throw cause;
				});
			/**
			 * Only reaches the fiber when RivetKit fails to begin; a later
			 * commit failure surfaces through the commit statement instead.
			 */
			done.catch((cause: unknown) =>
				resume(
					Effect.fail(
						toSqlError(
							cause,
							"Failed to begin transaction",
							"transaction",
						),
					),
				),
			);
		}),
		(transaction) => transaction.rollbackUnlessCommitted,
	).pipe(Effect.map((transaction) => transaction.connection));

/**
 * Builds an `effect/sql` client on an actor's RivetKit database (`c.db`).
 *
 * `withTransaction` runs through RivetKit's `db.transaction`, never through
 * `BEGIN` statements, so it does not interleave with other actor work.
 * Nested `withTransaction` calls use savepoints inside that transaction.
 *
 * RivetKit returns each row as an object keyed by column name, so duplicate
 * column names merge into one: `SELECT a.id, b.id` gives one `id`, also from
 * `.values`. Alias them (`SELECT a.id AS a_id, b.id AS b_id`).
 */
export const makeSqlClient = Effect.fnUntraced(function* (
	database: RivetkitDatabase,
): Effect.fn.Return<SqlClient.SqlClient> {
	/**
	 * Effect 4.0 has no `driver` option and later versions require it, so the
	 * options are not an inline literal that 4.0 would reject.
	 */
	const options = {
		acquirer: Effect.succeed(makeConnection(database)),
		transactionAcquirer: beginRivetkitTransaction(database),
		compiler: Statement.makeCompilerSqlite(),
		driver: "rivetkit",
		spanAttributes,
		beginTransaction: beginStatement,
		commit: commitStatement,
		rollback: rollbackStatement,
	};
	const client = yield* SqlClient.make(options).pipe(
		Effect.provideServiceEffect(Reactivity.Reactivity, Reactivity.make),
	);
	const withTransaction = client.withTransaction;

	return Object.assign(client, {
		withTransaction: <A, E, R>(effect: Effect.Effect<A, E, R>) =>
			withTransaction(effect).pipe(
				Effect.catchDefect((defect) =>
					defect instanceof CommitFailure
						? Effect.fail(defect.error)
						: Effect.die(defect),
				),
			),
	});
});
