// RACE RUNNER (Race Demo)
// Demonstrates: Race (parallel first-wins) for timeout patterns
// One actor per race task - actor key is the task ID

import { actor, event } from "rivetkit";
import { Loop, workflow } from "rivetkit/workflow";
import { actorCtx } from "./_helpers.ts";

export type RaceTask = {
	id: string;
	workDurationMs: number;
	timeoutMs: number;
	status: "running" | "work_won" | "timeout_won";
	result?: string;
	startedAt: number;
	completedAt?: number;
	actualDurationMs?: number;
};

type State = RaceTask;

export type RaceTaskInput = {
	workDurationMs?: number;
	timeoutMs?: number;
};

export const race = actor({
	createState: (c, input?: RaceTaskInput): RaceTask => ({
		id: c.key[0] as string,
		workDurationMs: input?.workDurationMs ?? 2000,
		timeoutMs: input?.timeoutMs ?? 3000,
		status: "running",
		startedAt: Date.now(),
	}),
	events: {
		raceStarted: event<RaceTask>(),
		raceCompleted: event<RaceTask>(),
	},

	actions: {
		getTask: (c): RaceTask => c.state,
	},

	run: workflow(async (ctx) => {
		await ctx.loop("race-loop", async (loopCtx) => {
				const c = actorCtx<State>(loopCtx);

				// Get durations inside a step since state is only available in steps
				const { workDurationMs, timeoutMs, taskId } = await loopCtx.step(
					"start-race",
					async () => {
						ctx.log.info({
							msg: "starting race",
							taskId: c.state.id,
							workDurationMs: c.state.workDurationMs,
							timeoutMs: c.state.timeoutMs,
						});
						c.broadcast("raceStarted", c.state);
						return {
							workDurationMs: c.state.workDurationMs,
							timeoutMs: c.state.timeoutMs,
							taskId: c.state.id,
						};
					}
				);

				const { winner, value } = await loopCtx.race("work-vs-timeout", [
					{
						name: "work",
						run: async (branchCtx) => {
							await branchCtx.sleep("simulate-work", workDurationMs);
							return await branchCtx.step("complete-work", async () => {
								return `Result for task ${taskId}`;
							});
						},
					},
					{
						name: "timeout",
						run: async (branchCtx) => {
							await branchCtx.sleep("timeout-wait", timeoutMs);
							return null;
						},
					},
				]);

				await loopCtx.step("save-result", async () => {
					c.state.completedAt = Date.now();
					c.state.actualDurationMs = c.state.completedAt - c.state.startedAt;

					if (winner === "work") {
						c.state.status = "work_won";
						c.state.result = value as string;
						ctx.log.info({
							msg: "work completed before timeout",
							taskId: c.state.id,
							durationMs: c.state.actualDurationMs,
						});
					} else {
						c.state.status = "timeout_won";
						ctx.log.info({
							msg: "timeout won the race",
							taskId: c.state.id,
							durationMs: c.state.actualDurationMs,
						});
					}

					c.broadcast("raceCompleted", c.state);
				});

				return Loop.break(undefined);
			});
	}),
});
