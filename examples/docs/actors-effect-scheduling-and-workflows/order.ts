import { Action, db, Workflow } from "@rivetkit/effect";
import { DateTime, Effect, Schema } from "effect";

export const GetStatus = Action.make("GetStatus", {
	success: Schema.String,
});

export const Order = Workflow.make("Order", {
	actions: [GetStatus],
});

export const OrderLive = Order.toLayer(
	Effect.gen(function* () {
		const sql = yield* Order.sql;

		const setStatus = Effect.fn("Order.setStatus")(function* (
			status: string,
		) {
			yield* sql`UPDATE orders SET status = ${status} WHERE id = 1`;
		});

		// Stands in for a call to a payment provider.
		const charge = Effect.fn("Order.charge")(function* () {
			yield* setStatus("paid");
			return yield* DateTime.now;
		});

		return Effect.gen(function* () {
			const paidAt = yield* Workflow.step("charge", charge(), {
				schema: Schema.DateTimeUtc,
				maxRetries: 5,
				timeout: "1 minute",
			});
			yield* Workflow.sleepUntil(
				"wait-for-shipping",
				DateTime.add(paidAt, { hours: 1 }),
			);
			yield* Workflow.step("ship", setStatus("shipped"));
		});
	}),
	{
		db: db({
			onMigrate: async (client) => {
				await client.execute(
					"CREATE TABLE IF NOT EXISTS orders (id INTEGER PRIMARY KEY CHECK (id = 1), status TEXT NOT NULL)",
				);
				await client.execute(
					"INSERT OR IGNORE INTO orders (id, status) VALUES (1, 'new')",
				);
			},
		}),
		actions: Effect.gen(function* () {
			const sql = yield* Order.sql;

			return Order.of({
				GetStatus: () =>
					sql<{
						status: string;
					}>`SELECT status FROM orders WHERE id = 1`.pipe(
						Effect.map(([{ status }]) => status),
						Effect.orDie,
					),
			});
		}),
	},
);
