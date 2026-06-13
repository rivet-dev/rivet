import { Actor, State } from "@rivetkit/effect";
import { Effect, Schema } from "effect";
import { Counter } from "./api.ts";

// --- Actor Implementation ---

// `.toLayer` produces a Layer that registers this actor with the `Registry`
// service in context. The first parameter is a `wake` function that runs once
// when the actor awakes and returns the action handlers.
export const CounterLive = Counter.toLayer(
	Effect.fnUntraced(function* ({ rawRivetkitContext, state }) {
		return Counter.of({
			Increment: Effect.fnUntraced(function* ({ payload }) {
				// Access the actor's persisted `state` with a `SubscriptionRef`-like API.
				const next = yield* State.updateAndGet(state, (current) => ({
					count: current.count + payload.amount,
				})).pipe(Effect.orDie);

				// Broadcast the new value to every connected client.
				rawRivetkitContext.broadcast("newCount", next.count);

				return next.count;
			}),
			GetCount: () =>
				State.get(state).pipe(
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
