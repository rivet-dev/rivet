import { Effect } from "effect";
import { Counter } from "./api";

export const program = Effect.gen(function* () {
	const counters = yield* Counter.client;

	// Compound key: [org, room]
	const counter = counters.getOrCreate(["org-acme", "general"]);
	yield* counter.Increment({ amount: 1 });
});
