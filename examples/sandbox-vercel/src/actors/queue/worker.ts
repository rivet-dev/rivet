import { actor } from "rivetkit";

export interface WorkerState {
	status: "idle" | "running";
	processed: number;
	lastJob: unknown;
}

export const worker = actor({
	state: {
		status: "idle" as "idle" | "running",
		processed: 0,
		lastJob: null as unknown,
	},
	async run(c) {
		c.state.status = "running";
		c.broadcast("statusChanged", {
			status: c.state.status,
			processed: c.state.processed,
		});

		while (!c.aborted) {
			const job = await c.queue.next("jobs", { timeout: 1000 });
			if (job) {
				c.state.processed += 1;
				c.state.lastJob = job.body;
				c.broadcast("jobProcessed", {
					processed: c.state.processed,
					job: job.body,
				});
			}
		}

		c.state.status = "idle";
	},
	actions: {
		getState(c): WorkerState {
			return {
				status: c.state.status,
				processed: c.state.processed,
				lastJob: c.state.lastJob,
			};
		},
	},
});
