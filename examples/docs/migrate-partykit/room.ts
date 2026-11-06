import { actor, event, setup } from "rivetkit";
import { db } from "rivetkit/db";

type ChatMessage = {
	id: number;
	sender: string;
	text: string;
};

interface Member {
	name: string;
	team: string;
}

// One chatRoom actor per room. The room ID is the actor's key.
export const chatRoom = actor({
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

	events: {
		message: event<ChatMessage>(),
		joined: event<{ name: string }>(),
		left: event<{ name: string }>(),
		typing: event<{ sender: string }>(),
		teamMessage: event<{ text: string }>(),
	},

	// docs:start connections
	// Live connections, keyed by connection ID
	createVars: () => ({ members: new Map<string, Member>() }),

	onConnect: (c, conn) => {
		// One-off action calls without params also pass through this hook
		if (!conn.params) return;
		const { name, team } = conn.params as { name: string; team: string };
		c.vars.members.set(conn.id, { name, team });
		c.broadcast("joined", { name });
	},

	onDisconnect: (c, conn) => {
		const member = c.vars.members.get(conn.id);
		c.vars.members.delete(conn.id);
		if (member) c.broadcast("left", { name: member.name });
	},
	// docs:end connections

	actions: {
		// docs:start broadcast
		// Replaces onMessage: each message type becomes an action
		sendMessage: async (c, text: string) => {
			const sender = c.vars.members.get(c.conn.id)?.name ?? "anonymous";
			const rows = await c.db.execute<{ id: number }>(
				"INSERT INTO messages (sender, text) VALUES (?, ?) RETURNING id",
				sender,
				text,
			);
			const message = { id: rows[0].id, sender, text };

			// room.broadcast(message): send to every connection
			c.broadcast("message", message);
			return message;
		},

		sendTyping: (c) => {
			const sender = c.vars.members.get(c.conn.id)?.name ?? "anonymous";

			// room.broadcast(message, [sender.id]): skip the sender
			for (const [id, conn] of c.conns) {
				if (id !== c.conn.id) conn.send("typing", { sender });
			}
		},
		// docs:end broadcast

		// docs:start tags
		// room.getConnections(tag): filter the tracked connections
		sendToTeam: (c, team: string, text: string) => {
			for (const [id, member] of c.vars.members) {
				if (member.team === team) {
					c.conns.get(id)?.send("teamMessage", { text });
				}
			}
		},

		onlineCount: (c) => c.vars.members.size,
		// docs:end tags

		getHistory: async (c) => {
			return await c.db.execute<ChatMessage>(
				"SELECT id, sender, text FROM messages ORDER BY id DESC LIMIT 50",
			);
		},
	},
});

export const registry = setup({ use: { chatRoom } });
registry.start();
