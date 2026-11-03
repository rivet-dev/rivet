import { actor, setup } from "rivetkit";
import { db } from "rivetkit/db";

// A chat room Actor. Every room key gets its own instance with its own SQLite
// database, which survives restarts and redeploys.
export const chatRoom = actor({
	// Create the messages table the first time a room starts.
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS messages (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					sender TEXT NOT NULL,
					text TEXT NOT NULL
				)
			`);
		},
	}),

	actions: {
		// Save a message, then push it to everyone connected to this room.
		sendMessage: async (c, sender: string, text: string) => {
			await c.db.execute(
				"INSERT INTO messages (sender, text) VALUES (?, ?)",
				sender,
				text,
			);

			c.broadcast("newMessage", { sender, text });
		},

		// Read the room's history, oldest first.
		getHistory: async (c) => {
			return (await c.db.execute(
				"SELECT sender, text FROM messages ORDER BY id",
			)) as { sender: string; text: string }[];
		},
	},
});

// Register the Actor and start serving it on http://localhost:6420.
export const registry = setup({ use: { chatRoom } });
registry.start();
