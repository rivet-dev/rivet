import { actor } from "rivetkit";
import { db, type RawAccess } from "rivetkit/db";

interface Worker {
  // Set when new jobs arrive while a drain is already running
  pending: boolean;
  draining: Promise<void> | null;
}

interface WorkerContext {
  db: RawAccess;
  vars: { worker: Worker };
  keepAwake<T>(promise: Promise<T>): Promise<T>;
  log: { error(fields: Record<string, unknown>): void };
}

// Start a drain unless one is already running. `keepAwake` stops the actor
// from sleeping until every job is processed.
function startDrain(c: WorkerContext): void {
  c.vars.worker.pending = true;
  if (c.vars.worker.draining) return;

  c.vars.worker.draining = c
    .keepAwake(drainJobs(c))
    .catch((error) => {
      // The failed job stays in the table and runs on the next drain
      c.log.error({ msg: "job failed", error });
    })
    .finally(() => {
      c.vars.worker.draining = null;
      // Catch jobs enqueued after the loop last checked `pending`
      if (c.vars.worker.pending) startDrain(c);
    });
}

async function drainJobs(c: WorkerContext): Promise<void> {
  while (c.vars.worker.pending) {
    c.vars.worker.pending = false;

    while (true) {
      const jobs = await c.db.execute<{ id: number; email: string }>(
        "SELECT id, email FROM jobs ORDER BY id LIMIT 1",
      );
      const job = jobs[0];
      if (!job) break;

      await sendWelcomeEmail(job.email);

      // Delete only after the work succeeds, so a crash retries the job
      await c.db.execute("DELETE FROM jobs WHERE id = ?", job.id);
    }
  }
}

const mailer = actor({
  db: db({
    onMigrate: async (db) => {
      await db.execute(`
        CREATE TABLE IF NOT EXISTS jobs (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          email TEXT NOT NULL
        )
      `);
    },
  }),
  createVars: (): { worker: Worker } => ({
    worker: { pending: false, draining: null },
  }),

  // Resume jobs that were still pending when the actor last slept or crashed
  onWake: (c) => startDrain(c),

  actions: {
    // The job is durable once the insert resolves, like a queue send
    enqueue: async (c, email: string) => {
      await c.db.execute("INSERT INTO jobs (email) VALUES (?)", email);
      startDrain(c);
    },
    pendingCount: async (c) => {
      const rows = await c.db.execute<{ count: number }>(
        "SELECT COUNT(*) AS count FROM jobs",
      );
      return rows[0].count;
    },
  },
});

async function sendWelcomeEmail(email: string): Promise<void> {
  console.log("sending welcome email", email);
}
