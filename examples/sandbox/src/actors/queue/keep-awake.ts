import { actor } from "rivetkit";

export interface CurrentTask {
	id: string;
	startedAt: number;
	durationMs: number;
}

export interface CompletedTask {
	id: string;
	completedAt: number;
}

export interface KeepAwakeState {
	currentTask: CurrentTask | null;
	completedTasks: CompletedTask[];
}

export const keepAwake = actor({
	state: {
		currentTask: null as CurrentTask | null,
		completedTasks: [] as CompletedTask[],
	},
	async run(c) {
		while (!c.abortSignal.aborted) {
			const job = await c.queue.next("tasks", { timeout: 1000 });
			if (job) {
				const taskId = crypto.randomUUID();
				const { durationMs } = job.body as { durationMs: number };

				c.state.currentTask = {
					id: taskId,
					startedAt: Date.now(),
					durationMs,
				};
				c.broadcast("taskStarted", c.state.currentTask);

				// Wrap long-running work in keepAwake so actor doesn't sleep
				await c.keepAwake(
					new Promise((resolve) => setTimeout(resolve, durationMs)),
				);

				c.state.completedTasks.push({ id: taskId, completedAt: Date.now() });
				c.state.currentTask = null;
				c.broadcast("taskCompleted", {
					taskId,
					completedTasks: c.state.completedTasks,
				});
			}
		}
	},
	actions: {
		getState(c): KeepAwakeState {
			return {
				currentTask: c.state.currentTask,
				completedTasks: c.state.completedTasks,
			};
		},
		clearTasks(c) {
			c.state.completedTasks = [];
			c.broadcast("taskCompleted", {
				taskId: null,
				completedTasks: [],
			});
		},
	},
	options: {
		sleepTimeout: 2000,
	},
});
