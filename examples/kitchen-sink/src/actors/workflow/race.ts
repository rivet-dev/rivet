// RACE RUNNER (Race Demo)
// Demonstrates: Race (parallel first-wins) for timeout patterns
// One actor per race task - actor key is the task ID

import { actor, event } from "rivetkit";
import { Loop, workflow } from "rivetkit/workflow";
import {
	loadRecord,
	recordDb,
	saveRecord,
	updateRecord,
} from "./record-store.ts";

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

export type RaceTaskInput = {
	workDurationMs?: number;
	timeoutMs?: number;
};

export const race = actor({
	db: recordDb(),
	onCreate: async (c, input?: RaceTaskInput) => {
		await saveRecord<RaceTask>(c.db, {
			id: c.key[0] as string,
			workDurationMs: input?.workDurationMs ?? 2000,
			timeoutMs: input?.timeoutMs ?? 3000,
			status: "running",
			startedAt: Date.now(),
		});
	},
	events: {
		raceStarted: event<RaceTask>(),
		raceCompleted: event<RaceTask>(),
	},

	actions: {
		getTask: (c): Promise<RaceTask> => loadRecord<RaceTask>(c.db),
	},

	run: workflow(async (ctx) => {
		await ctx.loop("race-loop", async (loopCtx) => {
			// Read the durations inside a step since the database is only available in steps
			const { workDurationMs, timeoutMs, taskId } = await loopCtx.step(
				"start-race",
				async (c) => {
					const task = await loadRecord<RaceTask>(c.db);
					c.log.info({
						msg: "starting race",
						taskId: task.id,
						workDurationMs: task.workDurationMs,
						timeoutMs: task.timeoutMs,
					});
					c.broadcast("raceStarted", task);
					return {
						workDurationMs: task.workDurationMs,
						timeoutMs: task.timeoutMs,
						taskId: task.id,
					};
				},
			);

			const { winner, value } = await loopCtx.race("work-vs-timeout", [
				{
					name: "work",
					run: async (branchCtx) => {
						await branchCtx.sleep("simulate-work", workDurationMs);
						return await branchCtx.step(
							"complete-work",
							async () => {
								return `Result for task ${taskId}`;
							},
						);
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

			await loopCtx.step("save-result", async (c) => {
				const updated = await updateRecord<RaceTask>(c.db, (task) => {
					task.completedAt = Date.now();
					task.actualDurationMs = task.completedAt - task.startedAt;

					if (winner === "work") {
						task.status = "work_won";
						task.result = value as string;
						c.log.info({
							msg: "work completed before timeout",
							taskId: task.id,
							durationMs: task.actualDurationMs,
						});
					} else {
						task.status = "timeout_won";
						c.log.info({
							msg: "timeout won the race",
							taskId: task.id,
							durationMs: task.actualDurationMs,
						});
					}
				});
				c.broadcast("raceCompleted", updated);
			});

			return Loop.break(undefined);
		});
	}),
});
