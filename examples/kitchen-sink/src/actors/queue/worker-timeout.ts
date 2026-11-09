import { actor, event, queue } from "rivetkit";
import { db } from "rivetkit/db";

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

type WorkerTimeoutRow = {
	processed: number;
	ticks: number;
	last_tick_at: number | null;
	last_job_id: string | null;
	last_job_payload: string | null;
	timeout_ms: number;
};

export const workerTimeout = actor({
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS worker_progress (
					id INTEGER PRIMARY KEY CHECK (id = 1),
					processed INTEGER NOT NULL DEFAULT 0,
					ticks INTEGER NOT NULL DEFAULT 0,
					last_tick_at INTEGER,
					last_job_id TEXT,
					last_job_payload TEXT,
					timeout_ms INTEGER NOT NULL
				);
			`);
			await db.execute(
				"INSERT OR IGNORE INTO worker_progress (id, timeout_ms) VALUES (1, ?)",
				DEFAULT_TIMEOUT_MS,
			);
		},
	}),
	// The status only describes the current run handler, so it lives in memory.
	createVars: () => ({ status: "idle" as WorkerTimeoutState["status"] }),
	events: {
		tick: event<{ ticks: number; at: number }>(),
		jobProcessed: event<{ processed: number; job: WorkerTimeoutJob }>(),
	},
	queues: {
		jobs: queue<WorkerTimeoutJob>(),
	},
	run: async (c) => {
		c.vars.status = "running";

		while (!c.aborted) {
			// Read the timeout on every pass so `setTimeoutMs` applies to the next wait.
			const [settings] = await c.db.execute<{ timeout_ms: number }>(
				"SELECT timeout_ms FROM worker_progress WHERE id = 1",
			);
			const message = await c.queue.next({
				names: ["jobs"],
				timeout: settings.timeout_ms,
			});

			if (!message) {
				const at = Date.now();
				const [progress] = await c.db.execute<{ ticks: number }>(
					"UPDATE worker_progress SET ticks = ticks + 1, last_tick_at = ? WHERE id = 1 RETURNING ticks",
					at,
				);
				c.broadcast("tick", {
					ticks: progress.ticks,
					at,
				});
				continue;
			}

			const [progress] = await c.db.execute<{ processed: number }>(
				"UPDATE worker_progress SET processed = processed + 1, last_job_id = ?, last_job_payload = ? WHERE id = 1 RETURNING processed",
				message.body.id,
				message.body.payload,
			);
			c.broadcast("jobProcessed", {
				processed: progress.processed,
				job: message.body,
			});
		}

		c.vars.status = "idle";
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
		setTimeoutMs: async (c, timeoutMs: number) => {
			const [row] = await c.db.execute<{ timeout_ms: number }>(
				"UPDATE worker_progress SET timeout_ms = ? WHERE id = 1 RETURNING timeout_ms",
				Math.max(100, Math.floor(timeoutMs)),
			);
			return row.timeout_ms;
		},
		getState: async (c): Promise<WorkerTimeoutState> => {
			const [row] = await c.db.execute<WorkerTimeoutRow>(
				"SELECT processed, ticks, last_tick_at, last_job_id, last_job_payload, timeout_ms FROM worker_progress WHERE id = 1",
			);
			const lastJob =
				row.last_job_id === null
					? null
					: {
							id: row.last_job_id,
							payload: row.last_job_payload ?? "",
						};
			return {
				status: c.vars.status,
				processed: row.processed,
				ticks: row.ticks,
				lastTickAt: row.last_tick_at,
				lastJob,
				timeoutMs: row.timeout_ms,
			};
		},
	},
});
