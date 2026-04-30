import { Schema } from "effect"
import { Actor, Action } from "@rivetkit/effect"

// --- Errors ---

export class CounterOverflowError extends Schema.TaggedErrorClass<CounterOverflowError>()(
	"CounterOverflowError",
	{ limit: Schema.Number },
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
// - Per-action middleware and annotations. Allows for Auth on some
//   actions but not others, timeout overrides...
//
// - Shared action protocols. A Ping health-check or GetMetrics
//   action defined once and composed into multiple actors.
export const Increment = Action.make("Increment", {
	payload: { amount: Schema.Number },
	success: Schema.Number,
	error: CounterOverflowError,
})

export const GetCount = Action.make("GetCount", {
	success: Schema.Number,
})

// --- Actor Definition ---

// The definition is the actor's public contract. It carries no
// implementation. Both server and client code import this;
// the implementation stays server-only.
export const Counter = Actor.make("Counter", {
	actions: [Increment, GetCount],
	options: {
		name: "Counter",	// Human-friendly display name
		icon: "comments",	// FontAwesome icon name
	},
})
