import { actor, UserError } from "rivetkit";
import { db, type RawAccess } from "rivetkit/db";

const MAX_ATTEMPTS = 3;
const RETRY_DELAY_MS = 2_000;

interface EmailTask {
	email: string;
	subject: string;
}

type TaskRow = {
	id: number;
	payload: string;
	attempts: number;
};

interface Worker {
	// Set when tasks arrive while a drain is already running
	pending: boolean;
	draining: Promise<void> | null;
}

interface WorkerContext {
	db: RawAccess;
	vars: { worker: Worker };
	schedule: {
		at(
			timestamp: number,
			action: string,
			...args: unknown[]
		): Promise<string>;
		list(): Promise<{ action: string; runAt: number }[]>;
	};
	keepAwake<T>(promise: Promise<T>): Promise<T>;
}

// Start a drain unless one is running. keepAwake holds the actor awake
// until the drain finishes.
function startDrain(c: WorkerContext): void {
	c.vars.worker.pending = true;
	if (c.vars.worker.draining !== null) return;

	c.vars.worker.draining = c
		.keepAwake(drainTasks(c))
		.catch((error) => console.error("drain failed", error))
		.finally(() => {
			c.vars.worker.draining = null;
		});
}

async function drainTasks(c: WorkerContext): Promise<void> {
	while (c.vars.worker.pending) {
		c.vars.worker.pending = false;

		while (true) {
			const [task] = await c.db.execute<TaskRow>(
				"SELECT id, payload, attempts FROM tasks WHERE run_at <= ? ORDER BY id LIMIT 1",
				Date.now(),
			);
			if (!task) break;
			await runTask(c, task);
		}
	}

	// Wake the actor when the next retry is due, unless a wake is already set
	const [next] = await c.db.execute<{ run_at: number | null }>(
		"SELECT MIN(run_at) AS run_at FROM tasks",
	);
	const due = next?.run_at;
	if (due == null) return;
	const scheduled = await c.schedule.list();
	if (!scheduled.some((s) => s.action === "drainDue" && s.runAt <= due)) {
		await c.schedule.at(due, "drainDue");
	}
}

async function runTask(c: WorkerContext, task: TaskRow): Promise<void> {
	// A task that used every attempt, for example by crashing the actor each
	// time, fails without running again
	if (task.attempts >= MAX_ATTEMPTS) {
		await failTask(c, task, "retries exhausted");
		return;
	}

	// Count the attempt before running, so a crash still uses it up
	await c.db.execute(
		"UPDATE tasks SET attempts = attempts + 1 WHERE id = ?",
		task.id,
	);
	const attempts = task.attempts + 1;

	try {
		await processEmail(JSON.parse(task.payload) as EmailTask);
	} catch (error) {
		if (attempts >= MAX_ATTEMPTS) {
			await failTask(c, task, String(error));
		} else {
			// Retry later with exponential backoff
			await c.db.execute(
				"UPDATE tasks SET run_at = ? WHERE id = ?",
				Date.now() + RETRY_DELAY_MS * 2 ** (attempts - 1),
				task.id,
			);
		}
		return;
	}

	// Delete only after the work succeeds
	await c.db.execute("DELETE FROM tasks WHERE id = ?", task.id);
}

// Move a task that ran out of attempts to failed_tasks
async function failTask(
	c: WorkerContext,
	task: TaskRow,
	error: string,
): Promise<void> {
	await c.db.transaction(async (tx) => {
		await tx.execute(
			"INSERT INTO failed_tasks (payload, error) VALUES (?, ?)",
			task.payload,
			error,
		);
		await tx.execute("DELETE FROM tasks WHERE id = ?", task.id);
	});
}

export const mailAgent = actor({
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
        CREATE TABLE IF NOT EXISTS tasks (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          payload TEXT NOT NULL,
          attempts INTEGER NOT NULL DEFAULT 0,
          run_at INTEGER NOT NULL
        )
      `);
			await db.execute(`
        CREATE TABLE IF NOT EXISTS failed_tasks (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          payload TEXT NOT NULL,
          error TEXT NOT NULL
        )
      `);
		},
	}),

	createVars: (): { worker: Worker } => ({
		worker: { pending: false, draining: null },
	}),

	// Resume tasks left over from before a sleep or crash
	onWake: (c) => startDrain(c),

	actions: {
		// Like this.queue("processEmail", payload): durable once the insert resolves
		queueEmail: async (c, task: EmailTask) => {
			await c.db.execute(
				"INSERT INTO tasks (payload, run_at) VALUES (?, ?)",
				JSON.stringify(task),
				Date.now(),
			);
			startDrain(c);
		},

		// Called by the schedule when a retry is due
		drainDue: (c) => {
			if (!c.scheduledFire) {
				throw new UserError("drainDue only runs from a schedule");
			}
			startDrain(c);
		},
	},
});

async function processEmail(task: EmailTask): Promise<void> {
	console.log("sending", task.subject, "to", task.email);
}
