import { Effect } from "effect";
import { Counter } from "./api";

export const program = Effect.fn(function* (actorId: string) {
	const counters = yield* Counter.client;

	// Creates the actor on the first call if it does not exist.
	const counter = counters.getOrCreate("my-counter");

	// Calls fail with `actor.not_found` if the actor does not exist.
	const existing = counters.get("my-counter");

	// Addresses an actor by its actor ID.
	const byId = counters.getForId(actorId);

	// Creates a new actor. Fails if an actor for the key exists.
	const created = yield* counters.create(["counter", crypto.randomUUID()]);

	return { counter, existing, byId, created };
});
