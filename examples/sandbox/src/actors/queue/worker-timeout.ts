import { actor, event, queue } from "rivetkit";

const DEFAULT_TIMEOUT_MS = 2_000;

export interface WorkerTimeoutJob {
	id: string;
	payload: string;
}

export interface WorkerTimeoutState {
	status: "idle" | "running";
	processed: number;
	ticks: number;
	lastTickAt: number | null;
	lastJob: WorkerTimeoutJob | null;
	timeoutMs: number;
}

export const workerTimeout = actor({
	state: {
		status: "idle" as "idle" | "running",
		processed: 0,
		ticks: 0,
		lastTickAt: null as number | null,
		lastJob: null as WorkerTimeoutJob | null,
		timeoutMs: DEFAULT_TIMEOUT_MS,
	},
	events: {
		tick: event<{ ticks: number; at: number }>(),
		jobProcessed: event<{ processed: number; job: WorkerTimeoutJob }>(),
	},
	queues: {
		jobs: queue<WorkerTimeoutJob>(),
	},
	run: async (c) => {
		c.state.status = "running";

		while (!c.aborted) {
			const [message] = await c.queue.next({
				names: ["jobs"],
				timeout: c.state.timeoutMs,
			});

			if (!message) {
				const at = Date.now();
				c.state.ticks += 1;
				c.state.lastTickAt = at;
				c.broadcast("tick", {
					ticks: c.state.ticks,
					at,
				});
				continue;
			}

			c.state.processed += 1;
			c.state.lastJob = message.body;
			c.broadcast("jobProcessed", {
				processed: c.state.processed,
				job: message.body,
			});
		}

		c.state.status = "idle";
	},
	actions: {
		enqueueJob: async (c, payload: string) => {
			const job = {
				id: crypto.randomUUID(),
				payload,
			};
			await c.queue.send("jobs", job);
			return job;
		},
		setTimeoutMs: (c, timeoutMs: number) => {
			c.state.timeoutMs = Math.max(100, Math.floor(timeoutMs));
			return c.state.timeoutMs;
		},
		getState: (c): WorkerTimeoutState => c.state,
	},
});
