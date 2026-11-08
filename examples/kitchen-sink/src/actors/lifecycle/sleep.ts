import { actor, event, type UniversalWebSocket } from "rivetkit";
import { db, type RawAccess } from "rivetkit/db";
import { promiseWithResolvers } from "rivetkit/utils";

export const SLEEP_TIMEOUT = 1000;

type LifecycleCountsRow = {
	start_count: number;
	sleep_count: number;
	request_count: number;
};

/** Creates a one-row table of lifecycle counts that survives sleep. */
function lifecycleCountsDb() {
	return db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS lifecycle_counts (
					id INTEGER PRIMARY KEY CHECK (id = 1),
					start_count INTEGER NOT NULL DEFAULT 0,
					sleep_count INTEGER NOT NULL DEFAULT 0,
					request_count INTEGER NOT NULL DEFAULT 0
				);
			`);
			await db.execute(
				"INSERT OR IGNORE INTO lifecycle_counts (id) VALUES (1)",
			);
		},
	});
}

/** Reads the lifecycle counts. */
async function loadCounts(database: RawAccess): Promise<LifecycleCountsRow> {
	const [counts] = await database.execute<LifecycleCountsRow>(
		"SELECT start_count, sleep_count, request_count FROM lifecycle_counts WHERE id = 1",
	);
	return counts;
}

/** Counts one start of the actor, including the first one. */
async function recordStart(database: RawAccess): Promise<void> {
	await database.execute(
		"UPDATE lifecycle_counts SET start_count = start_count + 1 WHERE id = 1",
	);
}

/** Counts one sleep of the actor. */
async function recordSleep(database: RawAccess): Promise<void> {
	await database.execute(
		"UPDATE lifecycle_counts SET sleep_count = sleep_count + 1 WHERE id = 1",
	);
}

export const sleep = actor({
	db: lifecycleCountsDb(),
	onWake: (c) => recordStart(c.db),
	onSleep: (c) => recordSleep(c.db),
	actions: {
		triggerSleep: (c) => {
			c.sleep();
		},
		getCounts: async (c) => {
			const counts = await loadCounts(c.db);
			return {
				startCount: counts.start_count,
				sleepCount: counts.sleep_count,
			};
		},
		setAlarm: async (c, duration: number) => {
			await c.schedule.after(duration, "onAlarm");
		},
		onAlarm: (c) => {
			c.log.info("alarm called");
		},
	},
	options: {
		sleepTimeout: SLEEP_TIMEOUT,
	},
});

export const sleepWithLongRpc = actor({
	db: lifecycleCountsDb(),
	createVars: () =>
		({}) as { longRunningResolve?: PromiseWithResolvers<void> },
	events: {
		waiting: event<[]>(),
	},
	onWake: (c) => recordStart(c.db),
	onSleep: (c) => recordSleep(c.db),
	actions: {
		getCounts: async (c) => {
			const counts = await loadCounts(c.db);
			return {
				startCount: counts.start_count,
				sleepCount: counts.sleep_count,
			};
		},
		longRunningRpc: async (c) => {
			c.log.info("starting long running rpc");
			const longRunningResolve = promiseWithResolvers<void>(() => {});
			c.vars.longRunningResolve = longRunningResolve;
			c.broadcast("waiting");
			await longRunningResolve.promise;
			c.log.info("finished long running rpc");
		},
		finishLongRunningRpc: (c) => c.vars.longRunningResolve?.resolve(),
	},
	options: {
		sleepTimeout: SLEEP_TIMEOUT,
	},
});

export const sleepWithRawHttp = actor({
	db: lifecycleCountsDb(),
	onWake: (c) => recordStart(c.db),
	onSleep: (c) => recordSleep(c.db),
	onRequest: async (c, request) => {
		await c.db.execute(
			"UPDATE lifecycle_counts SET request_count = request_count + 1 WHERE id = 1",
		);
		const url = new URL(request.url);

		if (url.pathname === "/long-request") {
			const duration = parseInt(
				url.searchParams.get("duration") || "1000",
				10,
			);
			c.log.info({ msg: "starting long fetch request", duration });
			await new Promise((resolve) => setTimeout(resolve, duration));
			c.log.info("finished long fetch request");
			return new Response(JSON.stringify({ completed: true }), {
				headers: { "Content-Type": "application/json" },
			});
		}

		return new Response("Not Found", { status: 404 });
	},
	actions: {
		getCounts: async (c) => {
			const counts = await loadCounts(c.db);
			return {
				startCount: counts.start_count,
				sleepCount: counts.sleep_count,
				requestCount: counts.request_count,
			};
		},
	},
	options: {
		sleepTimeout: SLEEP_TIMEOUT,
	},
});

export const sleepWithRawWebSocket = actor({
	db: lifecycleCountsDb(),
	// Open WebSockets do not survive sleep, so their count lives only in memory.
	createVars: () => ({ connectionCount: 0 }),
	onWake: (c) => recordStart(c.db),
	onSleep: (c) => recordSleep(c.db),
	onWebSocket: (c, websocket: UniversalWebSocket) => {
		c.vars.connectionCount += 1;
		c.log.info({
			msg: "websocket connected",
			connectionCount: c.vars.connectionCount,
		});

		websocket.send(
			JSON.stringify({
				type: "connected",
				connectionCount: c.vars.connectionCount,
			}),
		);

		const sendCounts = async () => {
			const counts = await loadCounts(c.db);
			websocket.send(
				JSON.stringify({
					type: "counts",
					startCount: counts.start_count,
					sleepCount: counts.sleep_count,
					connectionCount: c.vars.connectionCount,
				}),
			);
		};

		websocket.addEventListener("message", (event: any) => {
			const data = event.data;
			if (typeof data === "string") {
				try {
					const parsed = JSON.parse(data);
					if (parsed.type === "getCounts") {
						sendCounts().catch((error) => {
							c.log.warn({
								msg: "failed to read counts for websocket",
								error,
							});
						});
					} else if (parsed.type === "keepAlive") {
						// Just acknowledge to keep connection alive
						websocket.send(JSON.stringify({ type: "ack" }));
					}
				} catch {
					// Echo non-JSON messages
					websocket.send(data);
				}
			}
		});

		websocket.addEventListener("close", () => {
			c.vars.connectionCount -= 1;
			c.log.info({
				msg: "websocket disconnected",
				connectionCount: c.vars.connectionCount,
			});
		});
	},
	actions: {
		getCounts: async (c) => {
			const counts = await loadCounts(c.db);
			return {
				startCount: counts.start_count,
				sleepCount: counts.sleep_count,
				connectionCount: c.vars.connectionCount,
			};
		},
	},
	options: {
		sleepTimeout: SLEEP_TIMEOUT,
	},
});

export const sleepWithNoSleepOption = actor({
	db: lifecycleCountsDb(),
	onWake: (c) => recordStart(c.db),
	onSleep: (c) => recordSleep(c.db),
	actions: {
		getCounts: async (c) => {
			const counts = await loadCounts(c.db);
			return {
				startCount: counts.start_count,
				sleepCount: counts.sleep_count,
			};
		},
	},
	options: {
		sleepTimeout: SLEEP_TIMEOUT,
		noSleep: true,
	},
});
