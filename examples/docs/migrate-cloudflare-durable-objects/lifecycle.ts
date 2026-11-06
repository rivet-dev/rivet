import { actor } from "rivetkit";
import { db } from "rivetkit/db";

interface SessionInput {
	userId: string;
	ttlMs: number;
}

export const session = actor({
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
        CREATE TABLE IF NOT EXISTS session (
          id INTEGER PRIMARY KEY CHECK (id = 1),
          user_id TEXT NOT NULL,
          expires_at INTEGER NOT NULL
        )
      `);
		},
	}),

	// docs:start create
	// Runs once, when the actor is first created, with the input the creator passed
	onCreate: async (c, input: SessionInput) => {
		const expiresAt = Date.now() + input.ttlMs;
		await c.db.execute(
			"INSERT INTO session (id, user_id, expires_at) VALUES (1, ?, ?)",
			input.userId,
			expiresAt,
		);
		// TTL: destroy the actor and its data when the session expires
		await c.schedule.at(expiresAt, "expire");
	},
	// docs:end create

	// docs:start hooks
	// Runs on every start, including after sleep (like the constructor)
	onWake: (c) => {
		c.log.info({ msg: "session awake", key: c.key });
	},
	// Runs before the actor sleeps after being idle
	onSleep: (c) => {
		c.log.info({ msg: "session going to sleep" });
	},
	// Runs before the actor and its data are deleted
	onDestroy: (c) => {
		c.log.info({ msg: "session destroyed" });
	},
	options: {
		// How long an idle actor stays in memory before it sleeps
		sleepTimeout: 60_000,
	},
	// docs:end hooks

	actions: {
		// docs:start destroy
		// ctx.storage.deleteAll() plus letting the object go idle
		expire: (c) => {
			// Only the schedule may destroy the session, not a client
			if (!c.scheduledFire)
				throw new Error("only callable by the schedule");
			c.destroy();
		},
		// docs:end destroy
	},
});
