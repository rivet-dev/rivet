import { db } from "@rivetkit/effect";
import { Effect } from "effect";
import { Counter } from "./api";

export const CounterLive = Counter.toLayer(
	Effect.gen(function* () {
		const sql = yield* Counter.sql;

		return Counter.of({
			Increment: ({ payload }) =>
				sql<{
					count: number;
				}>`UPDATE counter SET count = count + ${payload.amount} WHERE id = 1 RETURNING count`.pipe(
					Effect.map(([{ count }]) => count),
					Effect.orDie,
				),
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
				await client.execute(
					"CREATE TABLE IF NOT EXISTS counter (id INTEGER PRIMARY KEY CHECK (id = 1), count INTEGER NOT NULL)",
				);
				await client.execute(
					"INSERT OR IGNORE INTO counter (id, count) VALUES (1, 0)",
				);
			},
		}),
	},
);
