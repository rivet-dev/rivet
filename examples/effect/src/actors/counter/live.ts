import { Effect, Ref } from "effect"
import { Counter, CounterOverflowError } from "./api.ts"

// --- Actor Implementation ---

// Counter.toLayer produces a Layer that registers this actor
// with whatever registry is in context. The Effect inside runs
// once per actor instance (not once per action call), so
// yielded refs are instance-scoped and survive across action
// calls within a wake. Finalizers run on sleep.
export const CounterLive = Counter.toLayer(
	// Wake scope (runs each wake, finalizers run on sleep)
	Effect.gen(function* () {
		// In-memory per-wake state. Resets on every wake; this v1
		// has no persistence. Replace with a persisted state ref
		// once Actor.State lands.
		const count = yield* Ref.make(0)

		yield* Effect.addFinalizer(() =>
			Ref.get(count).pipe(
				Effect.flatMap((n) => Effect.log(`sleeping count=${n}`)),
			),
		)

		// --- Action handlers (request-response) ---
		return Counter.of({
			Increment: ({ payload }) =>
				Effect.gen(function* () {
					const next = yield* Ref.updateAndGet(
						count,
						(n) => n + payload.amount,
					)
					if (next > 20) {
						return yield* new CounterOverflowError({ limit: 20 })
					}
					return next
				}),

			GetCount: () => Ref.get(count),
		})
	}),
)
