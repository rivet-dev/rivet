import { actor } from "rivetkit";
import { db, type RawAccess } from "rivetkit/db";

const MAX_MESSAGES = 50;

type ChatMessage = {
	id: string;
	text: string;
	timestamp: number;
};

export const rawWebSocketChatRoom = actor({
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS messages (
					seq INTEGER PRIMARY KEY AUTOINCREMENT,
					id TEXT NOT NULL,
					text TEXT NOT NULL,
					timestamp INTEGER NOT NULL
				);
			`);
		},
	}),
	// Load recent messages from SQLite into memory on every start.
	createVars: async (
		c,
	): Promise<{ messages: ChatMessage[]; sockets: Set<any> }> => {
		const rows = await c.db.execute<ChatMessage>(
			"SELECT id, text, timestamp FROM (SELECT seq, id, text, timestamp FROM messages ORDER BY seq DESC LIMIT ?) ORDER BY seq",
			MAX_MESSAGES,
		);
		return {
			messages: rows,
			sockets: new Set<any>(),
		};
	},
	onWebSocket(ctx, socket) {
		// Add socket to the set
		ctx.vars.sockets.add(socket);

		// Send recent messages to new connection
		socket.send(
			JSON.stringify({
				type: "init",
				messages: ctx.vars.messages,
			}),
		);

		// Handle incoming messages
		socket.addEventListener("message", (event: any) => {
			try {
				const data = JSON.parse(event.data);

				if (data.type === "message" && data.text) {
					const message = {
						id: crypto.randomUUID(),
						text: data.text,
						timestamp: Date.now(),
					};

					// Add to memory and write through to SQLite, keeping only
					// the last 50 messages.
					ctx.vars.messages.push(message);
					if (ctx.vars.messages.length > MAX_MESSAGES) {
						ctx.vars.messages.shift();
					}
					ctx.waitUntil(persistMessage(ctx.db, message));

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

/** Stores a chat message and prunes messages beyond the retained window. */
async function persistMessage(
	database: RawAccess,
	message: ChatMessage,
): Promise<void> {
	await database.execute(
		"INSERT INTO messages (id, text, timestamp) VALUES (?, ?, ?)",
		message.id,
		message.text,
		message.timestamp,
	);
	await database.execute(
		"DELETE FROM messages WHERE seq NOT IN (SELECT seq FROM messages ORDER BY seq DESC LIMIT ?)",
		MAX_MESSAGES,
	);
}
