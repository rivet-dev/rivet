import { Action, Actor } from "@rivetkit/effect";
import { Schema } from "effect";

// --- Errors ---

// A typed, schema-validated error. It travels through the action's error
// channel and arrives on the caller as a real tagged instance that can be
// matched with `Effect.catchTag`, not a string or an opaque exception.
export class NegativeAmountError extends Schema.TaggedErrorClass<NegativeAmountError>()(
	"NegativeAmountError",
	{
		amount: Schema.Number,
		message: Schema.String,
	},
) {}

// --- Actions ---

// Actions are standalone values with explicit `Schema` payloads,
// successes, and errors. The schemas validate encoded data end to end and
// control how values are encoded on the wire and decoded inside handlers.

export const Increment = Action.make("Increment", {
	payload: { amount: Schema.Number },
	success: Schema.Number,
	error: NegativeAmountError,
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
