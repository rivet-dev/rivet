import { actor, setup } from "rivetkit";
import { db } from "rivetkit/db";

type Message = {
	id: string;
	text: string;
	timestamp: number;
};

export const chatRoom = actor({
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS messages (
					seq INTEGER PRIMARY KEY AUTOINCREMENT,
					id TEXT NOT NULL,
					text TEXT NOT NULL,
					timestamp INTEGER NOT NULL
				)
			`);
		},
	}),
	createVars: () => {
		return {
			sockets: new Set<any>(),
		};
	},
	async onWebSocket(ctx, socket) {
		// Add socket to the set
		ctx.vars.sockets.add(socket);

		// Send recent messages to new connection
		const messages = (await ctx.db.execute(
			"SELECT id, text, timestamp FROM messages ORDER BY seq",
		)) as Message[];
		socket.send(
			JSON.stringify({
				type: "init",
				messages,
			}),
		);

		// Handle incoming messages
		socket.addEventListener("message", async (event: any) => {
			try {
				const data = JSON.parse(event.data);

				if (data.type === "message" && data.text) {
					const message: Message = {
						id: crypto.randomUUID(),
						text: data.text,
						timestamp: Date.now(),
					};

					// Persist the message to SQLite
					await ctx.db.execute(
						"INSERT INTO messages (id, text, timestamp) VALUES (?, ?, ?)",
						message.id,
						message.text,
						message.timestamp,
					);

					// Keep only last 50 messages
					await ctx.db.execute(
						"DELETE FROM messages WHERE seq NOT IN (SELECT seq FROM messages ORDER BY seq DESC LIMIT 50)",
					);

					// Broadcast to all connected clients
					const broadcast = JSON.stringify({
						type: "message",
						...message,
					});

					for (const ws of ctx.vars.sockets) {
						if (ws.readyState === 1) {
							// OPEN
							ws.send(broadcast);
						}
					}
				}
			} catch (e) {
				console.error("Failed to process message:", e);
			}
		});

		// Remove socket on close
		socket.addEventListener("close", () => {
			ctx.vars.sockets.delete(socket);
		});
	},
	actions: {},
});

// Register actors and start the server. https://rivet.dev/docs/setup
export const registry = setup({
	use: { chatRoom },
});

registry.start();
