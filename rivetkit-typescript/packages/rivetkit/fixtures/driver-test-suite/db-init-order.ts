import { actor } from "rivetkit";
import { db } from "@/common/database/mod";

// Verifies that onMigrate runs before createState/onCreate/createVars so the
// schema is queryable from those lifecycle hooks. The runtime should make
// `c.db` usable as soon as user code can read it.

export const dbInitOrderCreateStateActor = actor({
	createState: async (c, _input) => {
		const rows = await c.db.execute<{ count: number }>(
			"SELECT COUNT(*) as count FROM init_order_items",
		);
		return { count: rows[0]?.count ?? -1 };
	},
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS init_order_items (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					name TEXT NOT NULL
				)
			`);
		},
	}),
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
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS init_order_items (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					name TEXT NOT NULL
				)
			`);
		},
	}),
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
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS init_order_items (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					name TEXT NOT NULL
				)
			`);
		},
	}),
	actions: {
		getInitialCount: (c) => (c.vars as { initialCount: number }).initialCount,
	},
	options: {
		actionTimeout: 120_000,
		sleepTimeout: 100,
	},
});
