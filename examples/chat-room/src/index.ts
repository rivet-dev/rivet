import { actor, event, setup } from "rivetkit";
import { db } from "rivetkit/db";

export type Message = { sender: string; text: string; timestamp: number };

export const chatRoom = actor({
	// docs:start history
	// Persist chat history in the actor's SQLite database: https://rivet.dev/docs/actors/sqlite
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS messages (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					sender TEXT NOT NULL,
					text TEXT NOT NULL,
					timestamp INTEGER NOT NULL
				)
			`);
		},
	}),
	// docs:end history
	// docs:start broadcast
	events: {
		newMessage: event<Message>(),
	},
	// docs:end broadcast

	actions: {
		// docs:start broadcast
		// Callable functions from clients: https://rivet.dev/docs/actors/actions
		sendMessage: async (c, sender: string, text: string) => {
			const message: Message = { sender, text, timestamp: Date.now() };
			await c.db.execute(
				"INSERT INTO messages (sender, text, timestamp) VALUES (?, ?, ?)",
				sender,
				text,
				message.timestamp,
			);
			// Send events to all connected clients: https://rivet.dev/docs/actors/events
			c.broadcast("newMessage", message);
			return message;
		},
		// docs:end broadcast

		// docs:start history
		getHistory: async (c) => {
			const rows = await c.db.execute(
				"SELECT sender, text, timestamp FROM messages ORDER BY id ASC",
			);
			return rows as Message[];
		},
		// docs:end history
	},
});

// docs:start registry
// Register actors for use: https://rivet.dev/docs/setup
export const registry = setup({
	use: { chatRoom },
});

// Start the server on port 6420
registry.start();
// docs:end registry
