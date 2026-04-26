import { Effect, Queue, Ref, PubSub, Match } from "effect"
import { Actor } from "@rivetkit/effect"
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
		const messages = yield* Counter.Messages
		//    ^ MessageQueue<Reset | IncrementBy>
		const kv = yield* Actor.Kv
		const db = yield* Actor.Db

		// Ephemeral variable — reset on each wake, not persisted.
		const connectionsTotal = yield* Ref.make(0)

		yield* Effect.addFinalizer(() => Effect.log("sleeping"))

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

		// --- Message processing (durable queue) ---
		// Pull-based: the actor controls when to take the next message.
		// Forked into a scoped fiber, so it runs in the background and
		// is canceled on sleep.
		yield* Effect.gen(function* () {
			const msg = yield* Queue.take(messages)
			yield* Match.value(msg).pipe(
				Match.tag("Reset", () =>
					Effect.gen(function* () {
						yield* Ref.set(state, { count: 0 })
						yield* PubSub.publish(events.countChanged, 0)
					})
				),
				Match.tag("IncrementBy", ({ payload, complete }) =>
					Effect.gen(function* () {
						const next = yield* Ref.updateAndGet(state, (s) => ({
							count: s.count + payload.amount,
						}))
						yield* PubSub.publish(events.countChanged, next.count)
						yield* complete(next.count)
					})
				),
				Match.exhaustive,
			)
		}).pipe(Effect.forever, Effect.forkScoped)

		// --- Action handlers (request-response) ---
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
