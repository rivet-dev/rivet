import { actor, setup } from "rivetkit";
import { db } from "rivetkit/db";

// One user actor per user ID
export const user = actor({
	db: db({
		onMigrate: async (db) => {
			await db.execute(
				"CREATE TABLE IF NOT EXISTS rooms (room TEXT PRIMARY KEY)",
			);
		},
	}),
	actions: {
		addRoom: async (c, room: string) => {
			await c.db.execute(
				"INSERT OR IGNORE INTO rooms (room) VALUES (?)",
				room,
			);
		},
	},
});

// One room actor per room ID
export const room = actor({
	actions: {
		join: async (c, userId: string) => {
			// Replaces this.room.context.parties.user.get(userId).fetch(...)
			const client = c.client<typeof registry>();
			await client.user.getOrCreate([userId]).addRoom(c.key[0]);
		},
	},
});

// Replaces the "parties" map in partykit.json
export const registry = setup({ use: { user, room } });
