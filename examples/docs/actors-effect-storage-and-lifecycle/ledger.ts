import { Action, Actor, db } from "@rivetkit/effect";
import { Effect, Schema } from "effect";

export const Transfer = Action.make("Transfer", {
	payload: { from: Schema.String, to: Schema.String, amount: Schema.Number },
});

export const Ledger = Actor.make("Ledger", {
	actions: [Transfer],
});

export const LedgerLive = Ledger.toLayer(
	Effect.gen(function* () {
		const sql = yield* Ledger.sql;

		return Ledger.of({
			// docs:start transaction
			Transfer: ({ payload }) =>
				sql
					.withTransaction(
						Effect.gen(function* () {
							yield* sql`UPDATE accounts SET balance = balance - ${payload.amount} WHERE name = ${payload.from}`;
							yield* sql`UPDATE accounts SET balance = balance + ${payload.amount} WHERE name = ${payload.to}`;
						}),
					)
					.pipe(Effect.orDie),
			// docs:end transaction
		});
	}),
	{
		db: db(),
		// docs:start migrate
		onMigrate: ({ isNew }) =>
			Effect.gen(function* () {
				const sql = yield* Ledger.sql;
				yield* sql`CREATE TABLE IF NOT EXISTS accounts (name TEXT PRIMARY KEY, balance INTEGER NOT NULL)`;
				// `isNew` is `true` only on the start that creates the actor.
				if (isNew) {
					yield* sql`INSERT INTO accounts (name, balance) VALUES ('alice', 100), ('bob', 0)`;
				}
			}).pipe(Effect.orDie),
		// docs:end migrate
	},
);
