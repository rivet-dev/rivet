import { Effect } from "effect";
import { setup, event } from "rivetkit";
import { actor, Action, OnCreate, Log } from "@rivetkit/effect";

// A counter actor using Effect-TS for all actions and lifecycle hooks
export const counter = actor({
	state: {
		count: 0,
		lastUpdatedBy: "" as string,
	},
	events: {
		newCount: event<{ count: number; updatedBy: string }>(),
	},

	// Use Effect for lifecycle hooks
	onCreate: OnCreate.effect(function* (c) {
		yield* Log.info("Counter actor created", { actorId: c.actorId });
	}),

	actions: {
		// Use Effect.gen for action logic with typed errors and composition
		increment: Action.effect(function* (c, amount: number) {
			yield* Action.updateState(c, (s) => {
				s.count += amount;
				s.lastUpdatedBy = "increment";
			});

			const state = yield* Action.state(c);
			yield* Action.broadcast(c, "newCount", {
				count: state.count,
				updatedBy: state.lastUpdatedBy,
			});
			yield* Log.info("Counter incremented", { amount, newCount: state.count });

			return state.count;
		}),

		decrement: Action.effect(function* (c, amount: number) {
			yield* Action.updateState(c, (s) => {
				s.count -= amount;
				s.lastUpdatedBy = "decrement";
			});

			const state = yield* Action.state(c);
			yield* Action.broadcast(c, "newCount", {
				count: state.count,
				updatedBy: state.lastUpdatedBy,
			});
			yield* Log.info("Counter decremented", { amount, newCount: state.count });

			return state.count;
		}),

		// Demonstrates Effect composition — reset validates before updating
		reset: Action.effect(function* (c) {
			const state = yield* Action.state(c);

			// Skip if already zero
			if (state.count === 0) {
				yield* Log.debug("Counter already at zero, skipping reset");
				return 0;
			}

			yield* Action.updateState(c, (s) => {
				s.count = 0;
				s.lastUpdatedBy = "reset";
			});

			yield* Action.broadcast(c, "newCount", { count: 0, updatedBy: "reset" });
			yield* Log.info("Counter reset to zero");

			return 0;
		}),

		getCount: Action.effect(function* (c) {
			const state = yield* Action.state(c);
			return state.count;
		}),

		// Demonstrates using Effect.all for parallel operations
		batchIncrement: Action.effect(function* (c, amounts: number[]) {
			const total = amounts.reduce((sum, a) => sum + a, 0);

			yield* Action.updateState(c, (s) => {
				s.count += total;
				s.lastUpdatedBy = "batchIncrement";
			});

			const state = yield* Action.state(c);
			yield* Action.broadcast(c, "newCount", {
				count: state.count,
				updatedBy: state.lastUpdatedBy,
			});
			yield* Log.info("Batch increment applied", {
				operations: amounts.length,
				total,
				newCount: state.count,
			});

			return state.count;
		}),
	},
});

// Register actors
export const registry = setup({
	use: { counter },
});
