import { actor, event, UserError } from "rivetkit";
import { db } from "rivetkit/db";

type ConnParams = {
	authToken: string;
};

type Session = {
	userId: string;
	role: "member" | "admin";
};

async function authenticate(authToken: string): Promise<Session | null> {
	if (authToken === "admin-token") {
		return { userId: "admin-1", role: "admin" };
	}
	if (authToken === "member-token") {
		return { userId: "member-1", role: "member" };
	}
	return null;
}

// Resolve the caller's session, or reject the connection.
async function identify(params: ConnParams): Promise<Session> {
	const session = params.authToken ? await authenticate(params.authToken) : null;
	if (!session) {
		throw new UserError("Forbidden", { code: "forbidden" });
	}
	return session;
}

export const chatRoom = actor({
	db: db({
		onMigrate: async (db) => {
			await db.execute(
				"CREATE TABLE IF NOT EXISTS messages (id INTEGER PRIMARY KEY AUTOINCREMENT, user_id TEXT NOT NULL, text TEXT NOT NULL)",
			);
		},
	}),

	// The session for each live connection, keyed by connection ID
	createVars: () => ({ sessions: new Map<string, Session>() }),

	onBeforeConnect: async (_c, params: ConnParams) => {
		await identify(params);
	},

	onConnect: async (c, conn) => {
		c.vars.sessions.set(conn.id, await identify(conn.params));
	},

	onDisconnect: (c, conn) => {
		c.vars.sessions.delete(conn.id);
	},

	events: {
		messages: event<{ userId: string; text: string }>(),
		moderationLog: event<{ entry: string }>({
			canSubscribe: (c) => {
				if (c.conn && c.vars.sessions.get(c.conn.id)?.role === "admin") {
					return true;
				}
				return false;
			},
		}),
	},

	actions: {
		sendMessage: async (c, text: string) => {
			const session = c.conn ? c.vars.sessions.get(c.conn.id) : undefined;

			if (!session || (session.role !== "member" && session.role !== "admin")) {
				throw new UserError("Forbidden", { code: "forbidden" });
			}

			const message = { userId: session.userId, text };
			await c.db.execute(
				"INSERT INTO messages (user_id, text) VALUES (?, ?)",
				message.userId,
				message.text,
			);
			c.broadcast("messages", message);
		},
	},
});
