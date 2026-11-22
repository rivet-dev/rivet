/**
 * Async access to an actor's SQLite database for the cluster runtime.
 *
 * Rivet Actor SQLite is asynchronous, and while `db.transaction(callback)` is
 * open every other statement on the actor waits behind it. Statements issued
 * inside a transaction must therefore use the transaction handle, never the
 * actor's top-level handle, or they deadlock. `ActorSql` tracks the open
 * transaction per fiber, so code composed inside `transaction` reaches the
 * transaction handle and code on other fibers queues behind it.
 *
 * @internal
 */
import * as Context from "effect/Context";
import * as Effect from "effect/Effect";
import * as Exit from "effect/Exit";
import type { RawAccess } from "rivetkit/db";

/** @internal */
export type SqlRow = Record<string, unknown>;

/** @internal */
export interface ActorSql {
	/** Runs one statement on the current transaction, or on the database when none is open. */
	readonly exec: <Row extends object = SqlRow>(
		query: string,
		...args: ReadonlyArray<unknown>
	) => Effect.Effect<Array<Row>>;
	/**
	 * Runs `effect` inside one SQLite transaction. A failure or defect rolls the
	 * transaction back. A call nested inside an open transaction joins it.
	 */
	readonly transaction: <A, E, R>(
		effect: Effect.Effect<A, E, R>,
	) => Effect.Effect<A, E, R>;
	/** The actor's top-level database handle. */
	readonly db: RawAccess;
}

const CurrentTransaction = Context.Reference<RawAccess | undefined>(
	"@rivetkit/effect-cluster/internal/sql/CurrentTransaction",
	{ defaultValue: () => undefined },
);

class Rollback {
	constructor(readonly exit: Exit.Exit<unknown, unknown>) {}
}

/** @internal */
export const makeActorSql = (db: RawAccess): ActorSql => {
	const exec = <Row extends object = SqlRow>(
		query: string,
		...args: ReadonlyArray<unknown>
	): Effect.Effect<Array<Row>> =>
		Effect.withFiber((fiber) => {
			const target = fiber.getRef(CurrentTransaction) ?? db;
			return Effect.promise(
				() => target.execute(query, ...args) as Promise<Array<Row>>,
			);
		});

	const transaction = <A, E, R>(
		effect: Effect.Effect<A, E, R>,
	): Effect.Effect<A, E, R> =>
		Effect.withFiber((fiber) => {
			if (fiber.getRef(CurrentTransaction) !== undefined) return effect;
			return Effect.flatMap(Effect.context<R>(), (context) =>
				Effect.flatten(
					Effect.promise(async (): Promise<Exit.Exit<A, E>> => {
						try {
							return await db.transaction(async (tx) => {
								const exit = await Effect.runPromiseExit(
									effect.pipe(
										Effect.provideService(
											CurrentTransaction,
											tx,
										),
										Effect.provideContext(context),
									),
								);
								// Throwing makes RivetKit roll the transaction back.
								if (Exit.isFailure(exit))
									throw new Rollback(exit);
								return exit;
							});
						} catch (error) {
							if (error instanceof Rollback) {
								return error.exit as Exit.Exit<A, E>;
							}
							throw error;
						}
					}),
				),
			);
		});

	return { exec, transaction, db };
};
