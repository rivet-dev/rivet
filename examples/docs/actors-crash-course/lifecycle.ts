import { actor, event } from "rivetkit";
import { db } from "rivetkit/db";

interface RoomInput {
	roomName: string;
}

interface ConnInfo {
	userId: string;
	joinedAt: number;
}

const chatRoom = actor({
	events: {
		usersChanged: event<string[]>(),
	},

	// Database & vars initialization
	db: db({
		onMigrate: async (db) => {
			await db.execute(
				"CREATE TABLE IF NOT EXISTS room (id INTEGER PRIMARY KEY CHECK (id = 1), name TEXT NOT NULL)",
			);
		},
	}),
	createVars: () => ({
		startTime: Date.now(),
		users: new Map<string, ConnInfo>(),
	}),

	// Actor lifecycle
	onCreate: async (c, input: RoomInput) => {
		await c.db.execute("INSERT INTO room (id, name) VALUES (1, ?)", input.roomName);
		console.log("created", c.key);
	},
	onDestroy: (c) => console.log("destroyed"),
	onWake: (c) => console.log("actor started"),
	onSleep: (c) => console.log("actor sleeping"),

	// Connection lifecycle
	onBeforeConnect: (c, params) => {
		/* validate auth */
	},
	onConnect: (c, conn) => {
		const { userId } = conn.params as { userId: string };
		c.vars.users.set(conn.id, { userId, joinedAt: Date.now() });
		console.log("connected:", userId);
		c.broadcast("usersChanged", [...c.vars.users.values()].map((u) => u.userId));
	},
	onDisconnect: (c, conn) => {
		console.log("disconnected:", c.vars.users.get(conn.id)?.userId);
		c.vars.users.delete(conn.id);
		c.broadcast("usersChanged", [...c.vars.users.values()].map((u) => u.userId));
	},

	// Networking
	onRequest: (c, req) => new Response(JSON.stringify([...c.vars.users.values()])),
	onWebSocket: (c, socket) => socket.addEventListener("message", console.log),

	// Response transformation
	onBeforeActionResponse: <Out>(
		c: unknown,
		name: string,
		args: unknown[],
		output: Out,
	): Out => output,

	actions: {},
});
