import { Effect } from "effect";
import { Counter } from "./api";

export const program = Effect.gen(function* () {
	const counter = (yield* Counter.client).get("my-counter");

	const count = yield* counter.Increment({ amount: 1 }).pipe(
		// A declared error of the action.
		Effect.catchTag("CounterOverflow", ({ limit }) =>
			Effect.succeed(limit),
		),
		// All other failures.
		Effect.catchTag("RivetError", (error) =>
			error.reason._tag === "ActorNotFound"
				? Effect.succeed(0)
				: Effect.fail(error),
		),
	);
	yield* Effect.log(`Count: ${count}`);
});
