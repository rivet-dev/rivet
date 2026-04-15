import { actor, event, queue } from "rivetkit";

export interface WorkerJob {
	id: string;
	payload: string;
}

export interface WorkerState {
	status: "idle" | "running";
	processed: number;
	lastJob: WorkerJob | null;
}

export const worker = actor({
	state: {
		status: "idle" as "idle" | "running",
		processed: 0,
		lastJob: null as WorkerJob | null,
	},
	events: {
		statusChanged: event<{ status: "idle" | "running"; processed: number }>(),
		jobProcessed: event<{ processed: number; job: WorkerJob }>(),
	},
	queues: {
		jobs: queue<WorkerJob>(),
	},
	async run(c) {
		c.state.status = "running";
		c.broadcast("statusChanged", {
			status: c.state.status,
			processed: c.state.processed,
		});

		for await (const job of c.queue.iter()) {
			c.state.processed += 1;
			c.state.lastJob = job.body;
			c.broadcast("jobProcessed", {
				processed: c.state.processed,
				job: job.body,
			});
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
