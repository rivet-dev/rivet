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
  actions: {
    increment: {
      input: Schema.Struct({ amount: Schema.Number }),
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
      increment: ({ input }) =>
        Effect.gen(function* () {
          const next = yield* Ref.updateAndGet(state, (s) => ({
            count: s.count + input.amount,
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
