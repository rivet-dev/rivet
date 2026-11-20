import { actor } from "rivetkit";
import { db, type RawAccess } from "rivetkit/db";

// docs:start config
// Settings that Cloudflare reads from wrangler.toml
const BATCH_SIZE = 10; // max_batch_size
const MAX_RETRIES = 3; // max_retries
const RETRY_DELAY_MS = 5_000; // retry_delay, doubled on each retry
// docs:end config

interface Email {
	to: string;
	subject: string;
}

type Job = {
	id: number;
	body: string;
	attempts: number;
};

interface Vars {
	paused: boolean;
	// Set when new jobs arrive while a drain is already running
	pending: boolean;
	draining: Promise<void> | null;
}

interface QueueContext {
	db: RawAccess;
	vars: Vars;
	readonly aborted: boolean;
	keepAwake<T>(promise: Promise<T>): Promise<T>;
	schedule: {
		at(
			timestamp: number,
			action: string,
			...args: unknown[]
		): Promise<string>;
		list(): Promise<{ action: string; runAt: number }[]>;
	};
}

// docs:start consumer
// Stop starting new jobs when delivery is paused or the actor is shutting down
function stopped(c: QueueContext): boolean {
	return c.vars.paused || c.aborted;
}

// Start a drain unless one is already running. `keepAwake` stops the actor
// from sleeping until the drain finishes.
function startDrain(c: QueueContext): void {
	c.vars.pending = true;
	if (c.vars.draining !== null) return;

	c.vars.draining = c
		.keepAwake(drain(c))
		.catch((error) => console.error("drain failed", error))
		.finally(() => {
			c.vars.draining = null;
			// Jobs enqueued while the drain was finishing up start a new drain
			if (c.vars.pending && !stopped(c)) startDrain(c);
		});
}

async function drain(c: QueueContext): Promise<void> {
	while (c.vars.pending && !stopped(c)) {
		c.vars.pending = false;

		while (!stopped(c)) {
			// Read the next batch of due jobs, like one invocation of queue()
			const batch = await c.db.execute<Job>(
				"SELECT id, body, attempts FROM jobs WHERE run_at <= ? ORDER BY id LIMIT ?",
				Date.now(),
				BATCH_SIZE,
			);
			if (batch.length === 0) break;

			for (const job of batch) {
				if (stopped(c)) break;
				await processJob(c, job);
			}
		}
	}

	await scheduleNextWake(c);
}
// docs:end consumer

// docs:start retry
async function processJob(c: QueueContext, job: Job): Promise<void> {
	// A job that already used every delivery, for example because it crashed
	// the actor each time, goes straight to the dead letters
	if (job.attempts > MAX_RETRIES) {
		await deadLetter(c, job, "retries exhausted");
		return;
	}

	// Count the attempt before running the work, so a crash still uses it up
	await c.db.execute(
		"UPDATE jobs SET attempts = attempts + 1 WHERE id = ?",
		job.id,
	);

	let error: unknown = null;
	try {
		await sendEmail(JSON.parse(job.body) as Email);
	} catch (caught) {
		error = caught ?? new Error("job failed");
	}

	if (error === null) {
		// Acknowledge: delete the job only after the work succeeds
		await c.db.execute("DELETE FROM jobs WHERE id = ?", job.id);
		return;
	}

	const attempts = job.attempts + 1;
	if (attempts > MAX_RETRIES) {
		await deadLetter(c, job, String(error));
		return;
	}

	// Retry: make the job due again after an exponential backoff
	const delay = RETRY_DELAY_MS * 2 ** (attempts - 1);
	await c.db.execute(
		"UPDATE jobs SET run_at = ? WHERE id = ?",
		Date.now() + delay,
		job.id,
	);
}

// Move the job out of the queue in one transaction
async function deadLetter(
	c: QueueContext,
	job: Job,
	error: string,
): Promise<void> {
	await c.db.transaction(async (tx) => {
		await tx.execute(
			"INSERT OR REPLACE INTO dead_letters (id, body, error, failed_at) VALUES (?, ?, ?, ?)",
			job.id,
			job.body,
			error,
			Date.now(),
		);
		await tx.execute("DELETE FROM jobs WHERE id = ?", job.id);
	});
}
// docs:end retry

// docs:start wake
// Wake the actor when the next delayed or retried job is due. The actor can
// sleep until then.
async function scheduleNextWake(c: QueueContext): Promise<void> {
	if (c.vars.paused) return;

	const rows = await c.db.execute<{ next: number | null }>(
		"SELECT MIN(run_at) AS next FROM jobs",
	);
	const next = rows[0]?.next ?? null;
	if (next === null) return;

	// Schedules survive sleep, so skip if a drain is already due in time
	const scheduled = await c.schedule.list();
	if (
		scheduled.some(
			(event) => event.action === "drain" && event.runAt <= next,
		)
	) {
		return;
	}

	await c.schedule.at(next, "drain");
}
// docs:end wake

export const emailQueue = actor({
	// docs:start schema
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
        CREATE TABLE IF NOT EXISTS jobs (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          body TEXT NOT NULL,
          run_at INTEGER NOT NULL,
          attempts INTEGER NOT NULL DEFAULT 0
        );
      `);
			await db.execute(`
        CREATE TABLE IF NOT EXISTS dead_letters (
          id INTEGER PRIMARY KEY,
          body TEXT NOT NULL,
          error TEXT NOT NULL,
          failed_at INTEGER NOT NULL
        );
      `);
			await db.execute(`
        CREATE TABLE IF NOT EXISTS settings (
          id INTEGER PRIMARY KEY CHECK (id = 1),
          paused INTEGER NOT NULL
        );
      `);
			await db.execute(
				"INSERT OR IGNORE INTO settings (id, paused) VALUES (1, 0)",
			);
		},
	}),
	// docs:end schema

	// Load the paused flag into memory on every start
	createVars: async (c): Promise<Vars> => {
		const rows = await c.db.execute<{ paused: number }>(
			"SELECT paused FROM settings WHERE id = 1",
		);
		return {
			paused: rows[0]?.paused === 1,
			pending: false,
			draining: null,
		};
	},

	// docs:start resume-on-wake
	// Resume jobs that were pending when the actor last slept or crashed
	onWake: (c) => startDrain(c),
	// docs:end resume-on-wake

	actions: {
		// docs:start produce
		// The job is durable once the insert resolves, like a resolved send()
		enqueue: async (c, email: Email, options?: { delayMs?: number }) => {
			await c.db.execute(
				"INSERT INTO jobs (body, run_at) VALUES (?, ?)",
				JSON.stringify(email),
				Date.now() + (options?.delayMs ?? 0),
			);
			startDrain(c);
		},

		// Insert every job in one transaction, so the batch is all or nothing
		enqueueBatch: async (c, emails: Email[]) => {
			const now = Date.now();
			await c.db.transaction(async (tx) => {
				for (const email of emails) {
					await tx.execute(
						"INSERT INTO jobs (body, run_at) VALUES (?, ?)",
						JSON.stringify(email),
						now,
					);
				}
			});
			startDrain(c);
		},
		// docs:end produce

		// Called by the schedule when a delayed or retried job is due
		drain: (c) => startDrain(c),

		// docs:start admin
		pause: async (c) => {
			await c.db.execute("UPDATE settings SET paused = 1 WHERE id = 1");
			c.vars.paused = true;
		},

		resume: async (c) => {
			await c.db.execute("UPDATE settings SET paused = 0 WHERE id = 1");
			c.vars.paused = false;
			startDrain(c);
		},

		purge: async (c) => {
			await c.db.execute("DELETE FROM jobs");
		},

		backlog: async (c) => {
			const rows = await c.db.execute<{ count: number }>(
				"SELECT COUNT(*) AS count FROM jobs",
			);
			return rows[0].count;
		},
		// docs:end admin

		// docs:start dead-letters
		deadLetters: async (c) => {
			return await c.db.execute<{
				id: number;
				body: string;
				error: string;
			}>("SELECT id, body, error FROM dead_letters ORDER BY failed_at");
		},

		// Move a dead letter back into the queue with a fresh retry budget. It
		// keeps its ID, so side effects keyed on the job ID stay idempotent.
		redrive: async (c, id: number) => {
			await c.db.transaction(async (tx) => {
				await tx.execute(
					"INSERT INTO jobs (id, body, run_at) SELECT id, body, ? FROM dead_letters WHERE id = ?",
					Date.now(),
					id,
				);
				await tx.execute("DELETE FROM dead_letters WHERE id = ?", id);
			});
			startDrain(c);
		},
		// docs:end dead-letters
	},
});

async function sendEmail(email: Email): Promise<void> {
	console.log("sending email", email.to, email.subject);
}
