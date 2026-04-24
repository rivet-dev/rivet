import { Schema, Effect, Ref, PubSub } from "effect"
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
	payload: Schema.Struct({ amount: Schema.Number }),
	success: Schema.Number,
	error: CounterOverflowError,
})

export const GetCount = Action.make("GetCount", {
	success: Schema.Number,
})

// --- Actor Definition ---

// The definition is the actor's public contract: its name,
// state shape, event schemas, and action set. It carries no
// implementation, just types. Both server and client code
// import this; the implementation stays server-only.
export const Counter = Actor.make("Counter", {
	state: Schema.Struct({ count: Schema.Number }),
	events: { countChanged: Schema.Number },
	actions: [Increment, GetCount],
	options: {
		name: "Counter",	// Human-friendly display name
		icon: "comments", 	// FontAwesome icon name
	},
})

// --- Implementation ---

// Counter.toLayer produces a Layer that registers this actor
// with whatever registry is in context. The Effect inside runs
// once per actor instance (not once per action call), so
// yielded services like State and Events are instance-scoped.
export const CounterLive = Counter.toLayer(
	Effect.gen(function* () {
		// Actor-provided services are yielded from the Effect context.
		// They are scoped to this actor instance, not to individual
		// action calls. This means all action handlers below close
		// over the same state, events, kv, and db references.
		//
		// Because services come through the context (not a context
		// parameter like the current SDK's `c`), they are:
		//
		// - Visible in the type signature. The Effect's R channel
		//   declares exactly which services are required.
		//
		// - Swappable via layers. Tests can provide an in-memory KV
		//   or a mock DB without changing the actor code.
		const state = yield* Counter.State
		//    ^ SubscriptionRef<{ count: number }>
		const events = yield* Counter.Events
		//    ^ { countChanged: PubSub<number> }
		const kv = yield* Counter.Kv
		const db = yield* Counter.Db

		// Equivalent to current SDK's temporary variables
		const connectionsTotal = yield* Ref.make(0)

		// Finalizers run when the actor's scope closes
		yield* Effect.addFinalizer(() =>
			Effect.log("Counter destroyed? or/and sleep? (TBD)")
		)

		// Return the action implementations. Counter.of
		// type-checks each handler against its Action schema.
		return Counter.of({
			Increment: ({ payload }) =>
				Effect.gen(function* () {
					const next = yield* Ref.updateAndGet(state, (s) => ({
						count: s.count + payload.amount,
					}))
					if (next.count > 20) {
						return yield* new CounterOverflowError({ limit: 20 })
					}
					yield* PubSub.publish(events.countChanged, next.count)
					return next.count
				}),

			GetCount: () =>
				Ref.get(state).pipe(Effect.map((s) => s.count)),
		})
	}),
)
