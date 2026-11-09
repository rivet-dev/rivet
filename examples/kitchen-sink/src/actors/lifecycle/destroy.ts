import { actor } from "rivetkit";
import { db } from "rivetkit/db";
import type { registry } from "../../index.ts";

export const destroyObserver = actor({
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS destroyed_actors (
					actor_key TEXT PRIMARY KEY
				);
			`);
		},
	}),
	actions: {
		notifyDestroyed: async (c, actorKey: string) => {
			await c.db.execute(
				"INSERT OR IGNORE INTO destroyed_actors (actor_key) VALUES (?)",
				actorKey,
			);
		},
		wasDestroyed: async (c, actorKey: string) => {
			const rows = await c.db.execute(
				"SELECT 1 FROM destroyed_actors WHERE actor_key = ?",
				actorKey,
			);
			return rows.length > 0;
		},
		reset: async (c) => {
			await c.db.execute("DELETE FROM destroyed_actors");
		},
	},
});

export const destroyActor = actor({
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS destroy_value (
					id INTEGER PRIMARY KEY CHECK (id = 1),
					value INTEGER NOT NULL
				);
			`);
			await db.execute(
				"INSERT OR IGNORE INTO destroy_value (id, value) VALUES (1, 0)",
			);
		},
	}),
	onDestroy: async (c) => {
		// Report the actor key so the observer can confirm onDestroy ran.
		const client = c.client<typeof registry>();
		const observer = client.destroyObserver.getOrCreate(["observer"]);
		await observer.notifyDestroyed(c.key.join("/"));
	},
	actions: {
		setValue: async (c, newValue: number) => {
			await c.db.execute(
				"UPDATE destroy_value SET value = ? WHERE id = 1",
				newValue,
			);
			return newValue;
		},
		getValue: async (c) => {
			const rows = await c.db.execute<{ value: number }>(
				"SELECT value FROM destroy_value WHERE id = 1",
			);
			return rows[0].value;
		},
		destroy: (c) => {
			c.destroy();
		},
	},
});
