import { actor, event, setup } from "rivetkit";
import { db } from "rivetkit/db";

export type Message = {
	id: string;
	sender: string;
	text: string;
	timestamp: number;
};

export const chatRoom = actor({
	// Durable data lives in the actor's embedded SQLite database: https://rivet.dev/docs/actors/sqlite
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS messages (
					seq INTEGER PRIMARY KEY AUTOINCREMENT,
					id TEXT NOT NULL,
					sender TEXT NOT NULL,
					text TEXT NOT NULL,
					timestamp INTEGER NOT NULL
				)
			`);
		},
	}),

	events: {
		newMessage: event<Message>(),
		messagesCleared: event<[]>(),
	},

	actions: {
		// Callable functions from clients: https://rivet.dev/docs/actors/actions
		sendMessage: async (c, sender: string, text: string) => {
			const message: Message = {
				id: crypto.randomUUID(),
				sender,
				text,
				timestamp: Date.now(),
			};
			// The message survives restarts once the insert completes
			await c.db.execute(
				"INSERT INTO messages (id, sender, text, timestamp) VALUES (?, ?, ?, ?)",
				message.id,
				message.sender,
				message.text,
				message.timestamp,
			);
			// Send events to all connected clients: https://rivet.dev/docs/actors/events
			c.broadcast("newMessage", message);
			return message;
		},

		// Returns all messages for initial loading
		getMessages: (c) =>
			c.db.execute<Message>(
				"SELECT id, sender, text, timestamp FROM messages ORDER BY seq",
			),

		// Clears all messages and notifies clients
		clearMessages: async (c) => {
			await c.db.execute("DELETE FROM messages");
			c.broadcast("messagesCleared");
			return { success: true };
		},
	},
});

// Register actors and start the server: https://rivet.dev/docs/setup
export const registry = setup({
	use: { chatRoom },
});

registry.start();
