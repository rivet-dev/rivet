import { actor, setup } from "rivetkit";
import { db } from "rivetkit/db";

// Coordinator: tracks chat rooms within an organization
export const chatRoomList = actor({
	db: db({
		onMigrate: async (db) => {
			await db.execute("CREATE TABLE IF NOT EXISTS rooms (name TEXT PRIMARY KEY)");
		},
	}),
	actions: {
		addRoom: async (c, name: string) => {
			// Create the chat room actor
			const client = c.client<typeof registry>();
			await client.chatRoom.create([c.key[0], name]);
			await c.db.execute("INSERT INTO rooms (name) VALUES (?)", name);
		},
		listRooms: async (c) => {
			const rows = (await c.db.execute("SELECT name FROM rooms")) as { name: string }[];
			return rows.map((row) => row.name);
		},
	},
});

// Data actor: handles a single chat room
export const chatRoom = actor({
	db: db({
		onMigrate: async (db) => {
			await db.execute(
				"CREATE TABLE IF NOT EXISTS messages (id INTEGER PRIMARY KEY AUTOINCREMENT, text TEXT NOT NULL)",
			);
		},
	}),
	actions: {
		send: async (c, msg: string) => {
			await c.db.execute("INSERT INTO messages (text) VALUES (?)", msg);
		},
	},
});

export const registry = setup({ use: { chatRoomList, chatRoom } });

registry.start();
