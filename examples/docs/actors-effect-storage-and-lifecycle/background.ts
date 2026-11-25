import { Action, Actor, db } from "@rivetkit/effect";
import { Effect, Ref, Schedule } from "effect";

export const Track = Action.make("Track");

export const Metrics = Actor.make("Metrics", {
	actions: [Track],
});

// Stands in for real work.
const warmCache = Effect.sleep("10 seconds");

export const MetricsLive = Metrics.toLayer(
	Effect.gen(function* () {
		const sql = yield* Metrics.sql;
		const pendingHits = yield* Ref.make(0);

		const flush = Effect.gen(function* () {
			const hits = yield* Ref.getAndSet(pendingHits, 0);
			yield* sql`UPDATE metrics SET hits = hits + ${hits} WHERE id = 1`;
		}).pipe(Effect.orDie);

		// Starts on each wake. Stops when the actor sleeps or is destroyed.
		yield* flush.pipe(
			Effect.repeat(Schedule.spaced("5 seconds")),
			Effect.forkScoped,
		);

		// The actor stays awake until this work ends.
		yield* warmCache.pipe(Actor.keepAwake, Effect.forkScoped);

		return Metrics.of({
			Track: () => Ref.update(pendingHits, (hits) => hits + 1),
		});
	}),
	{
		db: db({
			onMigrate: async (client) => {
				await client.execute(
					"CREATE TABLE IF NOT EXISTS metrics (id INTEGER PRIMARY KEY CHECK (id = 1), hits INTEGER NOT NULL)",
				);
				await client.execute(
					"INSERT OR IGNORE INTO metrics (id, hits) VALUES (1, 0)",
				);
			},
		}),
	},
);
