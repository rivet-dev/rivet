import { actor, setup, UserError } from "rivetkit";
import { db } from "rivetkit/db";

interface ConnParams {
	authToken: string;
}

interface Session {
	userId: string;
	role: "member" | "admin";
}

// Replace this with your session store or auth provider.
async function verifySession(authToken: string): Promise<Session | null> {
	if (authToken === "admin-token") return { userId: "u_1", role: "admin" };
	if (authToken === "member-token") return { userId: "u_2", role: "member" };
	return null;
}

// Resolve the caller's session, or reject the connection.
async function identify(params: ConnParams): Promise<Session> {
	const session = await verifySession(params.authToken);
	if (!session) {
		throw new UserError("Invalid token", { code: "invalid_token" });
	}
	return session;
}

export const document = actor({
	db: db({
		onMigrate: async (db) => {
			await db.execute(
				"CREATE TABLE IF NOT EXISTS document (id INTEGER PRIMARY KEY CHECK (id = 1), body TEXT NOT NULL)",
			);
			await db.execute("INSERT OR IGNORE INTO document (id, body) VALUES (1, '')");
		},
	}),

	// The session for each live connection, keyed by connection ID
	createVars: () => ({ sessions: new Map<string, Session>() }),

	// 1. Identify the caller at connect time. Throwing rejects the connection.
	onBeforeConnect: async (_c, params: ConnParams) => {
		await identify(params);
	},

	onConnect: async (c, conn) => {
		c.vars.sessions.set(conn.id, await identify(conn.params));
	},

	onDisconnect: (c, conn) => {
		c.vars.sessions.delete(conn.id);
	},

	actions: {
		read: async (c) => {
			const rows = (await c.db.execute("SELECT body FROM document WHERE id = 1")) as {
				body: string;
			}[];
			return rows[0].body;
		},

		// 2. Gate the operation on the identity you established.
		edit: async (c, body: string) => {
			if (c.vars.sessions.get(c.conn.id)?.role !== "admin") {
				throw new UserError("Admins only", { code: "forbidden" });
			}
			await c.db.execute("UPDATE document SET body = ? WHERE id = 1", body);
		},
	},
});

export const registry = setup({ use: { document } });
