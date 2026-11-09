import { actor, UserError } from "rivetkit";
import { db } from "rivetkit/db";

interface ConnParams {
	authToken: string;
}

interface Caller {
	userId: string;
	role: string;
}

// Example token validation function
async function validateToken(
	token: string,
	roomKey: string[],
): Promise<{ sub: string; role: string } | null> {
	// In production, verify JWT or call auth service
	if (token.length > 0 && roomKey.length > 0) {
		return { sub: "user-123", role: "member" };
	}
	return null;
}

// Resolve the caller for this room, or reject the connection.
async function identify(params: ConnParams, roomKey: string[]): Promise<Caller> {
	const payload = await validateToken(params.authToken, roomKey);
	if (!payload) {
		throw new UserError("Forbidden", { code: "forbidden" });
	}
	return { userId: payload.sub, role: payload.role };
}

const chatRoom = actor({
	db: db({
		onMigrate: async (db) => {
			await db.execute(
				"CREATE TABLE IF NOT EXISTS messages (id INTEGER PRIMARY KEY AUTOINCREMENT, user_id TEXT NOT NULL, text TEXT NOT NULL, timestamp INTEGER NOT NULL)",
			);
		},
	}),

	// The caller for each live connection, keyed by connection ID
	createVars: () => ({ callers: new Map<string, Caller>() }),

	onBeforeConnect: async (c, params: ConnParams) => {
		await identify(params, c.key);
	},

	onConnect: async (c, conn) => {
		c.vars.callers.set(conn.id, await identify(conn.params, c.key));
	},

	onDisconnect: (c, conn) => {
		c.vars.callers.delete(conn.id);
	},

	actions: {
		sendMessage: async (c, text: string) => {
			// Look up the caller by connection ID
			const caller = c.vars.callers.get(c.conn.id);

			if (caller?.role !== "member") {
				throw new UserError("Insufficient permissions", {
					code: "insufficient_permissions",
				});
			}

			await c.db.execute(
				"INSERT INTO messages (user_id, text, timestamp) VALUES (?, ?, ?)",
				caller.userId,
				text,
				Date.now(),
			);
			c.broadcast("newMessage", { userId: caller.userId, text });
		},
	},
});
