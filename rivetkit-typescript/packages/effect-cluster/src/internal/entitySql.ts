/**
 * The `effect/sql` client an entity actor hands to its handlers, and the gate
 * that keeps mailbox writes out of user transactions.
 *
 * Statements outside a transaction run on the actor's database handle and
 * queue behind any open transaction. `withTransaction` parks the body inside
 * RivetKit's `db.transaction(callback)` and routes its statements to the
 * transaction handle, so the body's writes commit or roll back together.
 *
 * Portions adapted from Effect (MIT License, Copyright (c) 2023 Effectful
 * Technologies Inc). See `LICENSE-effect`.
 *
 * @internal
 */
import * as Context from "effect/Context";
import * as Effect from "effect/Effect";
import { identity } from "effect/Function";
import * as Reactivity from "effect/reactivity/Reactivity";
import type * as Scope from "effect/Scope";
import * as Semaphore from "effect/Semaphore";
import * as Stream from "effect/Stream";
import * as Client from "effect/sql/SqlClient";
import type { Connection } from "effect/sql/SqlConnection";
import { classifySqliteError, SqlError } from "effect/sql/SqlError";
import * as Statement from "effect/sql/Statement";
import type { RawAccess } from "rivetkit/db";
import { ActorSqlClient } from "../ActorSqlClient.ts";

/** @internal */
export interface EntitySql {
	/** Builds the client for one handler build, closed with that build's scope. */
	readonly make: Effect.Effect<
		Context.Context<ActorSqlClient>,
		never,
		Scope.Scope
	>;
	/** Runs a mailbox write once no user transaction is open. */
	readonly guard: <A, E, R>(
		effect: Effect.Effect<A, E, R>,
	) => Effect.Effect<A, E, R>;
}

const InUserTransaction = Context.Reference<boolean>(
	"@rivetkit/effect-cluster/internal/entitySql/InUserTransaction",
	{ defaultValue: () => false },
);

// Set while a fiber holds the gate for mailbox work, so nested mailbox
// statements inside a mailbox transaction do not wait on themselves.
const InMailboxGate = Context.Reference<boolean>(
	"@rivetkit/effect-cluster/internal/entitySql/InMailboxGate",
	{ defaultValue: () => false },
);

const TransactionConnection = Context.Service<
	Client.TransactionConnection,
	Client.TransactionConnection.Service
>("@rivetkit/effect-cluster/internal/entitySql/TransactionConnection");

const sqlError = (cause: unknown, message: string, operation: string) =>
	new SqlError({
		reason: classifySqliteError(cause, { message, operation }),
	});

/** A connection whose statements go to the database or an open transaction. */
interface RoutedConnection extends Connection {
	target: RawAccess;
	/** Settles the open transaction callback, committing or rolling back. */
	finish: ((commit: boolean) => Promise<void>) | undefined;
}

const makeConnection = (db: RawAccess): RoutedConnection => {
	const run = (
		connection: RoutedConnection,
		sql: string,
		params: ReadonlyArray<unknown>,
	): Effect.Effect<ReadonlyArray<Record<string, unknown>>, SqlError> =>
		Effect.tryPromise({
			try: () => connection.target.execute(sql, ...params),
			catch: (cause) =>
				sqlError(cause, "Failed to execute statement", "execute"),
		});

	const connection: RoutedConnection = {
		target: db,
		finish: undefined,
		execute(sql, params, transformRows) {
			const rows = run(connection, sql, params);
			return transformRows ? Effect.map(rows, transformRows) : rows;
		},
		executeRaw(sql, params) {
			return run(connection, sql, params);
		},
		executeValues(sql, params) {
			return Effect.map(run(connection, sql, params), (rows) =>
				rows.map((row) => Object.values(row)),
			);
		},
		executeValuesUnprepared(sql, params) {
			return Effect.map(run(connection, sql, params), (rows) =>
				rows.map((row) => Object.values(row)),
			);
		},
		executeUnprepared(sql, params, transformRows) {
			const rows = run(connection, sql, params);
			return transformRows ? Effect.map(rows, transformRows) : rows;
		},
		executeStream(sql, params, transformRows) {
			return Stream.fromIterableEffect(run(connection, sql, params)).pipe(
				transformRows
					? Stream.mapArray((chunk) => transformRows(chunk) as any)
					: identity,
			);
		},
	};
	return connection;
};

/**
 * Opens `db.transaction` and parks its callback until `finish` is called, so
 * the transaction body runs on the calling fiber.
 */
const begin = (db: RawAccess, connection: RoutedConnection) =>
	Effect.callback<void, SqlError>((resume) => {
		// Settles once RivetKit has committed or rolled back the transaction.
		const settled = Promise.withResolvers<void>();
		const transaction = db.transaction(
			(tx) =>
				new Promise<void>((resolve, reject) => {
					connection.target = tx;
					connection.finish = (commit) => {
						connection.finish = undefined;
						connection.target = db;
						if (commit) resolve();
						else reject(new TransactionRolledBack());
						return settled.promise;
					};
					resume(Effect.void);
				}),
		);
		transaction.then(
			() => settled.resolve(),
			(cause) => {
				if (cause instanceof TransactionRolledBack) {
					settled.resolve();
					return;
				}
				const error = sqlError(
					cause,
					"Failed transaction",
					"transaction",
				);
				// Takes effect only when RivetKit rejects before the callback runs.
				resume(Effect.fail(error));
				settled.reject(error);
			},
		);
	});

class TransactionRolledBack extends Error {}

const finish = (connection: RoutedConnection, commit: boolean) =>
	Effect.suspend(() => {
		const done = connection.finish;
		if (done === undefined) return Effect.void;
		return Effect.tryPromise({
			try: () => done(commit),
			catch: (cause) =>
				cause instanceof SqlError
					? cause
					: sqlError(cause, "Failed transaction", "transaction"),
		});
	});

const savepoint = (
	connection: RoutedConnection,
	statement: string,
): Effect.Effect<void, SqlError> =>
	Effect.asVoid(connection.executeUnprepared(statement, [], undefined));

/** @internal */
export const makeActorSqlClient = (
	db: RawAccess,
): Effect.Effect<
	Client.SqlClient,
	never,
	Scope.Scope | Reactivity.Reactivity
> =>
	Effect.gen(function* () {
		const compiler = Statement.makeCompilerSqlite();
		const spanAttributes: ReadonlyArray<readonly [string, unknown]> = [
			["db.system.name", "sqlite"],
		];
		const shared = makeConnection(db);
		const acquirer = Effect.succeed(shared as Connection);
		const client = yield* Client.make({
			driver: "rivetkit-sqlite",
			acquirer,
			compiler,
			transactionService: TransactionConnection,
			spanAttributes,
		});
		const withTransaction = Client.makeWithTransaction({
			transactionService: TransactionConnection,
			spanAttributes,
			acquireConnection: () =>
				Effect.sync(() => makeConnection(db) as Connection),
			begin: (connection) => begin(db, connection as RoutedConnection),
			savepoint: (connection, id) =>
				savepoint(
					connection as RoutedConnection,
					`SAVEPOINT effect_sql_${id}`,
				),
			releaseSavepoint: (connection, id) =>
				savepoint(
					connection as RoutedConnection,
					`RELEASE SAVEPOINT effect_sql_${id}`,
				),
			commit: (connection) =>
				finish(connection as RoutedConnection, true),
			rollback: (connection) =>
				finish(connection as RoutedConnection, false),
			rollbackSavepoint: (connection, id) =>
				savepoint(
					connection as RoutedConnection,
					`ROLLBACK TO SAVEPOINT effect_sql_${id}`,
				),
		});
		return Object.assign(client, { withTransaction });
	});

/** @internal */
export const makeEntitySql = (db: RawAccess): EntitySql => {
	const gate = Semaphore.makeUnsafe(1);

	const make = Effect.map(
		Effect.provideServiceEffect(
			makeActorSqlClient(db),
			Reactivity.Reactivity,
			Reactivity.make,
		),
		(client) => {
			// Nested transactions reuse the outer transaction, so only the
			// outermost one takes the gate.
			const withTransaction = client.withTransaction;
			const gated: Client.SqlClient["withTransaction"] = (effect) =>
				Effect.withFiber((fiber) =>
					fiber.getRef(InUserTransaction)
						? withTransaction(effect)
						: Semaphore.withPermit(
								gate,
								withTransaction(
									Effect.provideService(
										effect,
										InUserTransaction,
										true,
									),
								),
							),
				);
			return Context.make(
				ActorSqlClient,
				Object.assign(client, { withTransaction: gated }),
			);
		},
	);

	return {
		make,
		guard: (effect) =>
			Effect.withFiber((fiber) =>
				fiber.getRef(InMailboxGate)
					? effect
					: Semaphore.withPermit(
							gate,
							Effect.provideService(effect, InMailboxGate, true),
						),
			),
	};
};
