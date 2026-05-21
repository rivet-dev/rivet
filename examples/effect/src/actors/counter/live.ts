import { Actor, State } from "@rivetkit/effect";
import { Effect, Schema } from "effect";
import { Counter, CounterOverflowError } from "./api.ts";

// --- Actor Implementation ---

// Counter.toLayer produces a Layer that registers this actor
// with whatever registry is in context. The Effect inside runs
// once per actor instance (not once per action call), so
// yielded refs are instance-scoped and survive across action
// calls within a wake. Finalizers run on sleep.
export const CounterLive = Counter.toLayer(
	// Wake scope (runs each wake, finalizers run on sleep)
	(wakeOptions) =>
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

			// `wakeOptions.state` is a `State` view over the persisted store.
			// `State.changes` exposes every state change commit as a stream.
			const state = wakeOptions.state;
			//    ^ State.State<{ count: number }>
			// const events = yield* Counter.Events
			//    // ^ { countChanged: PubSub<number> }
			// const messages = yield* Counter.Messages
			//    // ^ MessageQueue<Reset | IncrementBy>
			// const kv = yield* Actor.Kv
			// const db = yield* Actor.Db
			const address = yield* Actor.CurrentAddress;
			yield* Effect.log(
				`waking ${address.name}/${address.key.join(",")} actorId=${address.actorId}`,
			);

			yield* Effect.addFinalizer(() =>
				State.get(state).pipe(
					Effect.orDie,
					Effect.flatMap(({ count }) =>
						Effect.log(
							`sleeping ${address.name}/${address.key.join(",")} count=${count}`,
						),
					),
				),
			);

			// --- Message processing (not yet implemented) ---
			// Pull-based: the actor controls when to take the next message.
			// Forked into a scoped fiber, so it runs in the background and
			// is canceled on sleep. Re-enable once Counter.Messages lands.
			//
			// yield* Effect.gen(function* () {
			// 	const msg = yield* Queue.take(messages)
			// 	yield* Match.value(msg).pipe(
			// 		Match.tag("Reset", () =>
			// 			Effect.gen(function* () {
			// 				yield* State.set(state, 0)
			// 				yield* PubSub.publish(events.countChanged, 0)
			// 			})
			// 		),
			// 		Match.tag("IncrementBy", ({ payload, complete }) =>
			// 			Effect.gen(function* () {
			// 				const next = yield* State.updateAndGet(
			// 					state,
			// 					(s) => ({ count: s.count + payload.amount }),
			// 				)
			// 				yield* PubSub.publish(events.countChanged, next.count)
			// 				yield* complete(next.count)
			// 			})
			// 		),
			// 		Match.exhaustive,
			// 	)
			// }).pipe(Effect.forever, Effect.forkScoped)

			// --- Action handlers (request-response) ---
			return Counter.of({
				Increment: ({ payload }) =>
					Effect.gen(function* () {
						const { count: next } = yield* State.updateAndGet(
							state,
							(s) => ({ count: s.count + payload.amount }),
						);
						if (next > 20) {
							return yield* new CounterOverflowError({
								limit: 20,
								message: `count ${next} would exceed limit 20`,
							});
						}
						// yield* PubSub.publish(events.countChanged, next)
						return next;
					}),

				GetCount: () =>
					State.get(state).pipe(Effect.map((s) => s.count)),
			});
		}),
	{
		state: {
			schema: Schema.Struct({
				count: Schema.Number,
			}),
			initialValue: () => ({ count: 0 }),
		},
		name: "Counter", // Human-friendly display name
		icon: "comments", // FontAwesome icon name
	},
);
