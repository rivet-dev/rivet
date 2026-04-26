import { actor } from "rivetkit";
import { db } from "@/common/database/mod";
import { scheduleActorSleep } from "./schedule-sleep";

// Verifies that c.db is usable from every lifecycle hook that can be async,
// including teardown hooks. Schema is created in onMigrate so each callback
// only succeeds if onMigrate has already run.

const onSleepObservations = new Map<string, number>();
const onDestroyObservations = new Map<string, number>();

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
		getInitialCount: (c) => (c.vars as { initialCount: number }).initialCount,
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
		onSleepObservations.set(c.actorId, rows[0]?.count ?? -1);
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
		onDestroyObservations.set(c.actorId, rows[0]?.count ?? -1);
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
	actions: {
		getOnSleepCount: (_c, actorId: string) =>
			onSleepObservations.get(actorId) ?? -1,
		getOnDestroyCount: (_c, actorId: string) =>
			onDestroyObservations.get(actorId) ?? -1,
	},
});
