import { actor, event, queue } from "rivetkit";
import { db, type RawAccess } from "rivetkit/db";

export interface WorkerJob {
	id: string;
	payload: string;
}

export interface WorkerState {
	status: "idle" | "running";
	processed: number;
	lastJob: WorkerJob | null;
}

type WorkerProgressRow = {
	processed: number;
	last_job_id: string | null;
	last_job_payload: string | null;
};

/** Reads how many jobs the worker has processed and the last one it saw. */
async function loadProgress(
	database: RawAccess,
): Promise<Pick<WorkerState, "processed" | "lastJob">> {
	const [row] = await database.execute<WorkerProgressRow>(
		"SELECT processed, last_job_id, last_job_payload FROM worker_progress WHERE id = 1",
	);
	const lastJob =
		row.last_job_id === null
			? null
			: { id: row.last_job_id, payload: row.last_job_payload ?? "" };
	return { processed: row.processed, lastJob };
}

export const worker = actor({
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS worker_progress (
					id INTEGER PRIMARY KEY CHECK (id = 1),
					processed INTEGER NOT NULL DEFAULT 0,
					last_job_id TEXT,
					last_job_payload TEXT
				);
			`);
			await db.execute(
				"INSERT OR IGNORE INTO worker_progress (id) VALUES (1)",
			);
		},
	}),
	// The status only describes the current run handler, so it lives in memory.
	createVars: () => ({ status: "idle" as WorkerState["status"] }),
	events: {
		statusChanged: event<{
			status: "idle" | "running";
			processed: number;
		}>(),
		jobProcessed: event<{ processed: number; job: WorkerJob }>(),
	},
	queues: {
		jobs: queue<WorkerJob>(),
	},
	async run(c) {
		c.vars.status = "running";
		const { processed } = await loadProgress(c.db);
		c.broadcast("statusChanged", {
			status: c.vars.status,
			processed,
		});

		for await (const job of c.queue.iter()) {
			const [progress] = await c.db.execute<{ processed: number }>(
				"UPDATE worker_progress SET processed = processed + 1, last_job_id = ?, last_job_payload = ? WHERE id = 1 RETURNING processed",
				job.body.id,
				job.body.payload,
			);
			c.broadcast("jobProcessed", {
				processed: progress.processed,
				job: job.body,
			});
		}

		c.vars.status = "idle";
	},
	actions: {
		async getState(c): Promise<WorkerState> {
			const progress = await loadProgress(c.db);
			return { status: c.vars.status, ...progress };
		},
	},
});
