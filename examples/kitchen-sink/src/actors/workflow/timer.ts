// TIMER (Sleep Demo)
// Demonstrates: Durable sleep that survives restarts
// One actor per timer - actor key is the timer ID

import { actor, event } from "rivetkit";
import { Loop, workflow } from "rivetkit/workflow";

export type Timer = {
	id: string;
	name: string;
	durationMs: number;
	startedAt: number;
	completedAt?: number;
};

export type TimerInput = {
	name?: string;
	durationMs?: number;
};

export const timer = actor({
	createState: (c, input?: TimerInput): Timer => ({
		id: c.actorKey[0] as string,
		name: input?.name ?? "Timer",
		durationMs: input?.durationMs ?? 10000,
		startedAt: Date.now(),
	}),
	events: {
		timerStarted: event<Timer>(),
		timerCompleted: event<Timer>(),
	},

	actions: {
		getTimer: (c): Timer => c.state,
	},

	run: workflow(async (ctx) => {
		await ctx.loop("timer-loop", async (loopCtx) => {
			// Get duration inside a step since state is only available in steps
			const durationMs = await loopCtx.step(
				"start-timer",
				async (step) => {
					step.log.info({
						msg: "starting timer",
						timerId: step.state.id,
						durationMs: step.state.durationMs,
					});
					step.broadcast("timerStarted", step.state);
					return step.state.durationMs;
				},
			);

			await loopCtx.sleep("countdown", durationMs);

			await loopCtx.step("complete-timer", async (step) => {
				step.state.completedAt = Date.now();
				step.broadcast("timerCompleted", step.state);
				step.log.info({
					msg: "timer completed",
					timerId: step.state.id,
				});
			});

			return Loop.break(undefined);
		});
	}),

	options: {
		sleepTimeout: 1000,
	},
});
