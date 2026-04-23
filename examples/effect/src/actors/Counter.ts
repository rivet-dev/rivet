import { Schema, Effect, Ref, PubSub } from "effect"
import { Actor } from "@rivetkit/effect"

// --- Errors ---

export class CounterOverflowError extends Schema.TaggedError<CounterOverflowError>()(
	"CounterOverflowError",
	{ limit: Schema.Number },
) {}

// --- Definition ---

// The definition is the actor's public contract: its name,
// state shape, event schemas, and action set. It carries no
// implementation, just types. Both server and client code
// import this; the implementation stays server-only.
export const Counter = Actor.make("Counter", {
	state: Schema.Struct({ count: Schema.Number }),
	events: { countChanged: Schema.Number },
	// Actions use explicit schemas rather than inferring types from
	// the handler signature (like the current Rivet SDK does) because:
	//
	// 1. Runtime validation. Client-to-server is an untrusted boundary.
	//    Schemas let the server validate wire data with
	//    Schema.decodeUnknown before it reaches handler code. Handler
	//    inference erases types at runtime and trusts whatever arrives.
	//
	// 2. Contract separation. The definition can be imported by client
	//    code without pulling in server dependencies. It can also be
	//    published as a standalone package or satisfied by multiple
	//    implementations (real, test, mock).
	//
	// 3. Wire encoding control. Effect Schema distinguishes encoded
	//    (wire) and decoded (runtime) types, e.g. Schema.Date decodes
	//    a string into a Date. Handler inference only gives the decoded
	//    type.
	actions: {
		increment: {
			payload: Schema.Struct({ amount: Schema.Number }),
			success: Schema.Number,
			error: CounterOverflowError,
		},
		getCount: {
			success: Schema.Number,
		},
	},
})

// --- Implementation ---

// Counter.toLayer produces a Layer that registers this actor
// with whatever registry is in context. The Effect inside runs
// once per actor instance (not once per action call), so
// yielded services like State and Events are instance-scoped.
export const CounterLive = Counter.toLayer(
	Effect.gen(function* () {
		// Access actor-provided services
		const state = yield* Counter.State
		//    ^ SubscriptionRef<{ count: number }>
		const events = yield* Counter.Events
		//    ^ { countChanged: PubSub<number> }
		const kv = yield* Counter.Kv
		const db = yield* Counter.Db

		// Finalizers run when the actor's scope closes
		yield* Effect.addFinalizer(() =>
			Effect.log("Counter destroyed? or/and sleep? (TBD)")
		)

		// Return the action implementations. Counter.of
		// type-checks each handler against its Action schema.
		return Counter.of({
			increment: ({ payload }) =>
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

			getCount: () =>
				Ref.get(state).pipe(Effect.map((s) => s.count)),
		})
	}),
)
