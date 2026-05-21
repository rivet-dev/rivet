import { Action, Actor } from "@rivetkit/effect";
import { Schema } from "effect";

// --- Errors ---

export class CounterOverflowError extends Schema.TaggedErrorClass<CounterOverflowError>()(
	"CounterOverflowError",
	{
		limit: Schema.Number,
		message: Schema.String,
	},
) {}

// --- Actions ---

// Actions use explicit schemas rather than inferring types from
// the handler signature (like the current Rivet SDK does) because:
//
// - Runtime validation. Client-to-server is an untrusted boundary.
//    Schemas validate wire data before it reaches handler code.
//    Handler inference erases types at runtime and trusts whatever
//    arrives.
//
// - Wire encoding control. Effect Schema distinguishes encoded
//    (wire) and decoded (runtime) types, e.g. Schema.Date decodes
//    a string into a Date. Handler inference only gives the decoded
//    type.
//
// Actions are standalone values (vs. embedded in the actor
// definition) because:
//
// - Shared action protocols. A Ping health-check or GetMetrics
//   action defined once and composed into multiple actors.
export const Increment = Action.make("Increment", {
	payload: { amount: Schema.Number },
	success: Schema.Number,
	error: CounterOverflowError,
});

export const GetCount = Action.make("GetCount", {
	success: Schema.Number,
});

// --- Messages (not yet implemented) ---
//
// // Non-completable (fire-and-forget)
// export const Reset = Message.make("Reset", {
// 	payload: { reason: Schema.String },
// })
//
// // Completable (sender can await a typed response)
// export const IncrementBy = Message.make("IncrementBy", {
// 	payload: { amount: Schema.Number },
// 	success: Schema.Number,
// })

// --- Actor Definition ---

// The definition is the actor's public contract. It carries no
// implementation and no persisted-state schema (state is server-only,
// configured via `ActorState.make` + `toLayer(wake, { state })` in `live.ts`).
// Both server and client code import this; the implementation stays
// server-only.
export const Counter = Actor.make("Counter", {
	actions: [Increment, GetCount],
	// messages: [Reset, IncrementBy],	// durable, queued, background
	// events: { countChanged: Schema.Number },
});
