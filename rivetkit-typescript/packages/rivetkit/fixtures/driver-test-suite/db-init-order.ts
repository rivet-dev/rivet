import { actor } from "rivetkit";
import { db } from "@/common/database/mod";
import type { registry } from "./registry-static";
import { scheduleActorSleep } from "./schedule-sleep";

// Verifies that c.db is usable from every lifecycle hook that can be async,
// including teardown hooks. Schema is created in onMigrate so each callback
// only succeeds if onMigrate has already run. Teardown observations are
// pushed to the observer actor via the inline client so they survive
// per-actor process or worker isolation.

const OBSERVER_KEY = "db-init-order-observer";

const initOrderProvider = () =>
	db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS init_order_items (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					name TEXT NOT NULL
				)
			`);
		},
	});

export const dbInitOrderCreateStateActor = actor({
	createState: async (c, _input) => {
		const rows = await c.db.execute<{ count: number }>(
			"SELECT COUNT(*) as count FROM init_order_items",
		);
		return { count: rows[0]?.count ?? -1 };
	},
	db: initOrderProvider(),
	actions: {
		getInitialCount: (c) => (c.state as { count: number }).count,
	},
	options: {
		actionTimeout: 120_000,
		sleepTimeout: 100,
	},
});

export const dbInitOrderOnCreateActor = actor({
	state: { initialCount: -1 },
	onCreate: async (c, _input) => {
		const rows = await c.db.execute<{ count: number }>(
			"SELECT COUNT(*) as count FROM init_order_items",
		);
		c.state.initialCount = rows[0]?.count ?? -1;
	},
	db: initOrderProvider(),
	actions: {
		getInitialCount: (c) => c.state.initialCount,
	},
	options: {
		actionTimeout: 120_000,
		sleepTimeout: 100,
	},
});

export const dbInitOrderCreateVarsActor = actor({
	state: {},
	createVars: async (c) => {
		const rows = await c.db.execute<{ count: number }>(
			"SELECT COUNT(*) as count FROM init_order_items",
		);
		return { initialCount: rows[0]?.count ?? -1 };
	},
	db: initOrderProvider(),
	actions: {
		getInitialCount: (c) =>
			(c.vars as { initialCount: number }).initialCount,
	},
	options: {
		actionTimeout: 120_000,
		sleepTimeout: 100,
	},
});

export const dbInitOrderOnWakeActor = actor({
	state: {},
	onWake: async (c) => {
		await c.db.execute(
			"INSERT INTO init_order_items (name) VALUES (?)",
			"wake-event",
		);
	},
	db: initOrderProvider(),
	actions: {
		getWakeCount: async (c) => {
			const rows = await c.db.execute<{ count: number }>(
				"SELECT COUNT(*) as count FROM init_order_items WHERE name = 'wake-event'",
			);
			return rows[0]?.count ?? -1;
		},
		triggerSleep: (c) => {
			scheduleActorSleep(c);
		},
	},
	options: {
		actionTimeout: 120_000,
		sleepTimeout: 100,
	},
});

export const dbInitOrderOnSleepActor = actor({
	state: {},
	onSleep: async (c) => {
		const rows = await c.db.execute<{ count: number }>(
			"SELECT COUNT(*) as count FROM init_order_items",
		);
		const client = c.client<typeof registry>();
		await client.dbInitOrderObserver
			.getOrCreate([OBSERVER_KEY])
			.recordOnSleep(c.actorId, rows[0]?.count ?? -1);
		await c.db.execute(
			"INSERT INTO init_order_items (name) VALUES (?)",
			"sleep-event",
		);
	},
	db: initOrderProvider(),
	actions: {
		getActorId: (c) => c.actorId,
		insert: async (c, name: string) => {
			await c.db.execute(
				"INSERT INTO init_order_items (name) VALUES (?)",
				name,
			);
		},
		getSleepEventCount: async (c) => {
			const rows = await c.db.execute<{ count: number }>(
				"SELECT COUNT(*) as count FROM init_order_items WHERE name = 'sleep-event'",
			);
			return rows[0]?.count ?? -1;
		},
		triggerSleep: (c) => {
			scheduleActorSleep(c);
		},
	},
	options: {
		actionTimeout: 120_000,
		sleepTimeout: 100,
	},
});

export const dbInitOrderOnDestroyActor = actor({
	state: {},
	onDestroy: async (c) => {
		const rows = await c.db.execute<{ count: number }>(
			"SELECT COUNT(*) as count FROM init_order_items",
		);
		const client = c.client<typeof registry>();
		await client.dbInitOrderObserver
			.getOrCreate([OBSERVER_KEY])
			.recordOnDestroy(c.actorId, rows[0]?.count ?? -1);
	},
	db: initOrderProvider(),
	actions: {
		getActorId: (c) => c.actorId,
		insert: async (c, name: string) => {
			await c.db.execute(
				"INSERT INTO init_order_items (name) VALUES (?)",
				name,
			);
		},
		triggerDestroy: (c) => {
			c.destroy();
		},
	},
	options: {
		actionTimeout: 120_000,
	},
});

export const dbInitOrderObserver = actor({
	state: {
		onSleep: {} as Record<string, number>,
		onDestroy: {} as Record<string, number>,
	},
	actions: {
		recordOnSleep: (c, actorId: string, count: number) => {
			c.state.onSleep[actorId] = count;
		},
		recordOnDestroy: (c, actorId: string, count: number) => {
			c.state.onDestroy[actorId] = count;
		},
		getOnSleepCount: (c, actorId: string) => c.state.onSleep[actorId] ?? -1,
		getOnDestroyCount: (c, actorId: string) =>
			c.state.onDestroy[actorId] ?? -1,
	},
});
