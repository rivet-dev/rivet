// TIMER (Sleep Demo)
// Demonstrates: Durable sleep that survives restarts
// One actor per timer - actor key is the timer ID

import { actor, event } from "rivetkit";
import { Loop, workflow } from "rivetkit/workflow";
import { actorCtx } from "./_helpers.ts";

export type Timer = {
	id: string;
	name: string;
	durationMs: number;
	startedAt: number;
	completedAt?: number;
};

type State = Timer;

export type TimerInput = {
	name?: string;
	durationMs?: number;
};

export const timer = actor({
	createState: (c, input?: TimerInput): Timer => ({
		id: c.key[0] as string,
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
				const c = actorCtx<State>(loopCtx);

				// Get duration inside a step since state is only available in steps
				const durationMs = await loopCtx.step("start-timer", async () => {
					ctx.log.info({
						msg: "starting timer",
						timerId: c.state.id,
						durationMs: c.state.durationMs,
					});
					c.broadcast("timerStarted", c.state);
					return c.state.durationMs;
				});

				await loopCtx.sleep("countdown", durationMs);

				await loopCtx.step("complete-timer", async () => {
					c.state.completedAt = Date.now();
					c.broadcast("timerCompleted", c.state);
					ctx.log.info({ msg: "timer completed", timerId: c.state.id });
				});

				return Loop.break(undefined);
			});
	}),

	options: {
		sleepTimeout: 1000,
	},
});
