import { Effect, Fiber, Stream } from "effect";
import { Counter } from "./api";

export const program = Effect.gen(function* () {
	const counter = (yield* Counter.client).getOrCreate("my-counter");
	const connection = yield* counter.connect;

	// Start the stream before the calls, so it gets the events they cause.
	const changes = yield* connection.events.CountChanged.pipe(
		Stream.take(2),
		Stream.runCollect,
		Effect.forkScoped({ startImmediately: true }),
	);

	yield* connection.Increment({ amount: 1 });
	yield* connection.Increment({ amount: 2 });

	for (const { count } of yield* Fiber.join(changes)) {
		yield* Effect.log(`Count changed: ${count}`);
	}
}).pipe(Effect.scoped);
