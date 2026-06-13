import { Action, Actor } from "@rivetkit/effect";
import { Schema } from "effect";

// --- Actions ---

// Actions are standalone values with explicit `effect/Schema` payloads and
// successes. The schemas validate encoded data end to end and control how
// values are encoded on the wire and decoded inside handlers.

export const Increment = Action.make("Increment", {
	payload: { amount: Schema.Number },
	success: Schema.Number,
});

export const GetCount = Action.make("GetCount", {
	success: Schema.Number,
});

// --- Actor Definition ---

// The definition is the actor's public contract. It carries no implementation
// or server-only configuration, so it can be imported from client code without
// leaking server details.
export const Counter = Actor.make("Counter", {
	actions: [Increment, GetCount],
});
