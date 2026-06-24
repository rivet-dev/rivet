import { Actor } from "@rivetkit/effect";
import { Effect, Schema } from "effect";
import { Counter, NegativeAmountError } from "./api.ts";

// --- Actor Implementation ---

// `.toLayer` produces a Layer that registers this actor with the `Registry`
// service in context. The first parameter is a `wake` function that runs once
// when the actor awakes and returns the action handlers.
export const CounterLive = Counter.toLayer(
	Effect.fnUntraced(function* ({ rawRivetkitContext, state }) {
		return Counter.of({
			Increment: Effect.fnUntraced(function* ({ payload }) {
				// Reject before mutating, so the error path leaves state untouched.
				// The failure is a value in the typed error channel, not a throw.
				if (payload.amount < 0) {
					return yield* new NegativeAmountError({
						amount: payload.amount,
						message: `increment amount ${payload.amount} must not be negative`,
					});
				}

				// Access the actor's persisted `state` with a `SubscriptionRef`-like API.
				const next = yield* state
					.updateAndGet((current) => ({
						count: current.count + payload.amount,
					}))
					.pipe(Effect.orDie);

				// Broadcast the new value to every connected client.
				rawRivetkitContext.broadcast("newCount", next.count);

				return next.count;
			}),
			GetCount: () =>
				state.get.pipe(
					Effect.map((current) => current.count),
					Effect.orDie,
				),
		});
	}),
	{
		state: {
			schema: Schema.Struct({ count: Schema.Number }),
			initialValue: () => ({ count: 0 }),
		},
		name: "Counter", // Human-friendly display name
		icon: "calculator", // FontAwesome icon name
	},
);
