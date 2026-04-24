import { Cause, Effect, Exit, Ref, PubSub } from "effect"
import { Counter, CounterOverflowError } from "./api.ts"

// --- Actor Implementation ---

// Counter.toLayer produces a Layer that registers this actor
// with whatever registry is in context. The Effect inside runs
// once per actor instance (not once per action call), so
// yielded services like State and Events are instance-scoped.
export const CounterLive = Counter.toLayer(
	// Wake scope (runs each wake, finalizers run on sleep)
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

		yield* Counter.onCreate(
		)

		yield* Effect.addFinalizer((exit) =>
			Exit.match(exit, {
				onSuccess: () =>
					// Normal close = sleep
					Effect.log("sleeping"),
				onFailure: (cause) =>
					Cause.match(cause, {
						onInterrupt: () => Effect.log("destroyed"),
						onDie: (defect) => Effect.log("unexpected crash", defect),
					}),
			})
		)


		// Lifecycle hooks are just Effects that run at the right time.
		// onConnect receives the connection — its scope finalizer IS onDisconnect.
		yield* Counter.onConnect((conn) =>
			Effect.gen(function* () {
				yield* PubSub.publish(events.userJoined, conn.params.userId)

				// Finalizer runs on disconnect (or sleep for non-hibernatable).
				// This replaces onDisconnect — cleanup is co-located with setup.
				yield* Effect.addFinalizer(() =>
					Effect.log(`${conn.params.userId} disconnected`)
				)
			})
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
