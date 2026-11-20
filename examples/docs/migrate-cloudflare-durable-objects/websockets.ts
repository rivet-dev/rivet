import { actor, event, setup } from "rivetkit";
import { createClient } from "rivetkit/client";
import { db } from "rivetkit/db";

type Message = {
	user: string;
	text: string;
};

// docs:start actor
// acceptWebSocket + webSocketMessage + getWebSockets() broadcast
export const chatRoom = actor({
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
        CREATE TABLE IF NOT EXISTS messages (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          user TEXT NOT NULL,
          text TEXT NOT NULL
        )
      `);
		},
	}),
	events: {
		message: event<Message>(),
	},

	// Per-socket data you kept with serializeAttachment(), keyed by connection ID
	createVars: () => ({ users: new Map<string, string>() }),
	onConnect: (c, conn) => {
		// One-off action calls also run onConnect, with null params
		const params = conn.params as { user?: string } | null;
		c.vars.users.set(conn.id, params?.user ?? "anonymous");
	},
	onDisconnect: (c, conn) => {
		c.vars.users.delete(conn.id);
	},

	actions: {
		// Replaces parsing the message type inside webSocketMessage()
		send: async (c, text: string) => {
			const message = {
				user: c.vars.users.get(c.conn.id) ?? "anonymous",
				text,
			};
			await c.db.execute(
				"INSERT INTO messages (user, text) VALUES (?, ?)",
				message.user,
				message.text,
			);
			// Replaces looping over ctx.getWebSockets() and calling ws.send()
			c.broadcast("message", message);
		},
		history: async (c) => {
			return await c.db.execute<Message>(
				"SELECT user, text FROM messages ORDER BY id DESC LIMIT 50",
			);
		},
	},
});
// docs:end actor

export const registry = setup({ use: { chatRoom } });

// docs:start client
// new WebSocket(url): the client opens and reconnects the connection for you
const client = createClient<typeof registry>("http://localhost:6420");
const room = client.chatRoom.getOrCreate(["general"]).connect({ user: "ada" });
room.on("message", (message) =>
	console.log(`${message.user}: ${message.text}`),
);
await room.send("hello");
// docs:end client
