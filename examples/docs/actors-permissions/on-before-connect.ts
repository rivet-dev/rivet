import { actor, UserError } from "rivetkit";
import { db } from "rivetkit/db";

interface ConnParams {
	authToken: string;
}

// Example token validation function
async function validateToken(
	token: string,
	roomKey: string[],
): Promise<boolean> {
	// In production, verify JWT or call auth service
	return token.length > 0 && roomKey.length > 0;
}

const chatRoom = actor({
	db: db({
		onMigrate: async (db) => {
			await db.execute(
				"CREATE TABLE IF NOT EXISTS messages (id INTEGER PRIMARY KEY AUTOINCREMENT, text TEXT NOT NULL, timestamp INTEGER NOT NULL)",
			);
		},
	}),

	onBeforeConnect: async (c, params: ConnParams) => {
		const roomName = c.key;
		const isValid = await validateToken(params.authToken, roomName);
		if (!isValid) {
			throw new UserError("Forbidden", { code: "forbidden" });
		}
	},

	actions: {
		sendMessage: async (c, text: string) => {
			await c.db.execute(
				"INSERT INTO messages (text, timestamp) VALUES (?, ?)",
				text,
				Date.now(),
			);
		},
	},
});
