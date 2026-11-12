import { Actor } from "@rivetkit/effect";
import { Effect } from "effect";
import { db } from "rivetkit/db";
import { Counter, NegativeAmountError } from "./api.ts";

// --- Actor Implementation ---

// `.toLayer` produces a Layer that registers this actor with the `Registry`
// service in context. The first parameter is a `wake` function that runs once
// when the actor awakes and returns the action handlers.
export const CounterLive = Counter.toLayer(
	Effect.fnUntraced(function* ({ rawRivetkitContext }) {
		const readCount = Effect.tryPromise(async () => {
			const rows = await rawRivetkitContext.db.execute<{ count: number }>(
				"SELECT count FROM counter WHERE id = 1",
			);
			return rows[0]?.count ?? 0;
		}).pipe(Effect.orDie);

		return Counter.of({
			Increment: Effect.fnUntraced(function* ({ payload }) {
				// Reject before writing, so the error path leaves the count untouched.
				// The failure is a value in the typed error channel, not a throw.
				if (payload.amount < 0) {
					return yield* new NegativeAmountError({
						amount: payload.amount,
						message: `increment amount ${payload.amount} must not be negative`,
					});
				}

				// Persist the new count in the actor's SQLite database.
				yield* Effect.tryPromise(() =>
					rawRivetkitContext.db.execute(
						"UPDATE counter SET count = count + ? WHERE id = 1",
						payload.amount,
					),
				).pipe(Effect.orDie);
				const next = yield* readCount;

				// Broadcast the new value to every connected client.
				rawRivetkitContext.broadcast("newCount", next);

				return next;
			}),
			GetCount: () => readCount,
		});
	}),
	{
		db: db({
			onMigrate: async (client) => {
				await client.execute(`
					CREATE TABLE IF NOT EXISTS counter (
						id INTEGER PRIMARY KEY CHECK (id = 1),
						count INTEGER NOT NULL
					)
				`);
				await client.execute(
					"INSERT OR IGNORE INTO counter (id, count) VALUES (1, 0)",
				);
			},
		}),
		name: "Counter", // Human-friendly display name
		icon: "calculator", // FontAwesome icon name
	},
);
