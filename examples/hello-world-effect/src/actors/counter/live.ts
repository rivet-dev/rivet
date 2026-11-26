import { db } from "@rivetkit/effect";
import { Effect } from "effect";
import { CountChanged, Counter, NegativeAmountError } from "./api.ts";

// --- Actor Implementation ---

// `.toLayer` produces a Layer that registers this actor with the `Registry`
// service in context. The first parameter is a `wake` function that runs once
// when the actor awakes and returns the action handlers.
export const CounterLive = Counter.toLayer(
	Effect.fn(function* () {
		/**
		 * The actor's SQLite database, as an `effect/sql` client. Actors
		 * that declare `db` get one.
		 */
		const sql = yield* Counter.sql;
		// The events API for this actor's declared events.
		const events = yield* Counter.events;

		return Counter.of({
			Increment: Effect.fn(function* ({ payload }) {
				// Reject before writing, so the error path leaves the count untouched.
				// The failure is a value in the typed error channel, not a throw.
				if (payload.amount < 0) {
					return yield* new NegativeAmountError({
						amount: payload.amount,
						message: `increment amount ${payload.amount} must not be negative`,
					});
				}

				// Add the amount and read the new count in one atomic statement.
				const [{ count }] = yield* sql<{
					count: number;
				}>`UPDATE counter SET count = count + ${payload.amount} WHERE id = 1 RETURNING count`.pipe(
					Effect.orDie,
				);

				// Send the new count to every connected client.
				yield* events.broadcast(CountChanged, { count });

				return count;
			}),
			GetCount: () =>
				sql<{
					count: number;
				}>`SELECT count FROM counter WHERE id = 1`.pipe(
					Effect.map(([{ count }]) => count),
					Effect.orDie,
				),
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
