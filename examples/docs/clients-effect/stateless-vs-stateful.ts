import { Effect } from "effect";
import { Counter } from "./api";

export const program = Effect.gen(function* () {
	const counter = (yield* Counter.client).getOrCreate("my-counter");

	// Stateless: each call is independent.
	yield* counter.Increment({ amount: 1 });

	// Stateful: the connection stays open until the scope closes.
	const connection = yield* counter.connect;
	yield* connection.Increment({ amount: 1 });
}).pipe(Effect.scoped);
