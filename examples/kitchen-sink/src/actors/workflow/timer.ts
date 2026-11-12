// TIMER (Sleep Demo)
// Demonstrates: Durable sleep that survives restarts
// One actor per timer - actor key is the timer ID

import { actor, event } from "rivetkit";
import { Loop, workflow } from "rivetkit/workflow";
import {
	loadRecord,
	recordDb,
	saveRecord,
	updateRecord,
} from "./record-store.ts";

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
	db: recordDb(),
	onCreate: async (c, input?: TimerInput) => {
		await saveRecord<Timer>(c.db, {
			id: c.key[0] as string,
			name: input?.name ?? "Timer",
			durationMs: input?.durationMs ?? 10000,
			startedAt: Date.now(),
		});
	},
	events: {
		timerStarted: event<Timer>(),
		timerCompleted: event<Timer>(),
	},

	actions: {
		getTimer: (c): Promise<Timer> => loadRecord<Timer>(c.db),
	},

	run: workflow(async (ctx) => {
		await ctx.loop("timer-loop", async (loopCtx) => {
			// Read the duration inside a step since the database is only available in steps
			const durationMs = await loopCtx.step("start-timer", async (c) => {
				const timer = await loadRecord<Timer>(c.db);
				c.log.info({
					msg: "starting timer",
					timerId: timer.id,
					durationMs: timer.durationMs,
				});
				c.broadcast("timerStarted", timer);
				return timer.durationMs;
			});

			await loopCtx.sleep("countdown", durationMs);

			await loopCtx.step("complete-timer", async (c) => {
				const updated = await updateRecord<Timer>(c.db, (timer) => {
					timer.completedAt = Date.now();
				});
				c.broadcast("timerCompleted", updated);
				c.log.info({
					msg: "timer completed",
					timerId: updated.id,
				});
			});

			return Loop.break(undefined);
		});
	}),

	options: {
		sleepTimeout: 1000,
	},
});
