import { actor } from "rivetkit";
import { db, type RawAccess } from "rivetkit/db";

// A crash loses at most this much progress
const SNAPSHOT_INTERVAL_MS = 1_000;
const TICK_MS = 50;

type Ball = { id: string; x: number; y: number; vx: number; vy: number };

interface WorldVars {
	balls: Ball[];
	// `onWake` stores its loop here so `onSleep` can wait for it.
	loop: Promise<void> | null;
}

interface WorldContext {
	vars: WorldVars;
	db: RawAccess;
	aborted: boolean;
	abortSignal: AbortSignal;
}

const world = actor({
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
        CREATE TABLE IF NOT EXISTS balls (
          id TEXT PRIMARY KEY,
          x REAL NOT NULL,
          y REAL NOT NULL,
          vx REAL NOT NULL,
          vy REAL NOT NULL
        );
      `);
		},
	}),

	// The live simulation runs in memory, starting from the last snapshot
	createVars: async (c): Promise<WorldVars> => ({
		balls: await c.db.execute<Ball>("SELECT id, x, y, vx, vy FROM balls"),
		loop: null,
	}),

	onWake: (c) => {
		c.vars.loop = runWorld(c).catch((error) => {
			c.log.error({ msg: "world loop failed", error });
		});
	},

	// `c.abortSignal` fires on shutdown, so the loop saves and exits on its own
	onSleep: async (c) => {
		await c.vars.loop;
	},

	actions: {
		addBall: (c, ball: Ball) => {
			c.vars.balls.push(ball);
		},
		balls: (c) => c.vars.balls,
	},
});

async function runWorld(c: WorldContext): Promise<void> {
	let lastSnapshotAt = Date.now();
	while (!c.aborted) {
		await delay(TICK_MS, c.abortSignal);
		for (const ball of c.vars.balls) {
			ball.x += ball.vx;
			ball.y += ball.vy;
		}

		if (Date.now() - lastSnapshotAt >= SNAPSHOT_INTERVAL_MS) {
			await saveSnapshot(c);
			lastSnapshotAt = Date.now();
		}
	}

	// Save once more on shutdown so a graceful stop loses nothing
	await saveSnapshot(c);
}

// Replace the snapshot in one transaction so a crash never leaves half of one
async function saveSnapshot(c: WorldContext): Promise<void> {
	await c.db.transaction(
		async (tx) => {
			await tx.execute("DELETE FROM balls");
			for (const ball of c.vars.balls) {
				await tx.execute(
					"INSERT INTO balls (id, x, y, vx, vy) VALUES (?, ?, ?, ?, ?)",
					ball.id,
					ball.x,
					ball.y,
					ball.vx,
					ball.vy,
				);
			}
		},
		{ name: "save-snapshot" },
	);
}

// Resolve after `ms`, or as soon as the actor starts shutting down
function delay(ms: number, signal: AbortSignal): Promise<void> {
	return new Promise((resolve) => {
		if (signal.aborted) {
			resolve();
			return;
		}

		const finish = () => {
			clearTimeout(timer);
			signal.removeEventListener("abort", finish);
			resolve();
		};
		const timer = setTimeout(finish, ms);
		signal.addEventListener("abort", finish, { once: true });
	});
}
