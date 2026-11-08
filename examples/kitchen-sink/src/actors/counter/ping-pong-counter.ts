import { actor } from "rivetkit";
import { db } from "rivetkit/db";

export const pingPongCounter = actor({
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS ping_counter (
					id INTEGER PRIMARY KEY CHECK (id = 1),
					ping_count INTEGER NOT NULL
				);
			`);
			await db.execute(
				"INSERT OR IGNORE INTO ping_counter (id, ping_count) VALUES (1, 0)",
			);
		},
	}),
	onWebSocket(ctx, websocket) {
		websocket.addEventListener("message", async (event: any) => {
			const data = event.data;
			if (typeof data !== "string") return;

			let parsed: any;
			try {
				parsed = JSON.parse(data);
			} catch {
				return;
			}

			if (parsed?.type === "ping") {
				const [counter] = await ctx.db.execute<{ ping_count: number }>(
					"UPDATE ping_counter SET ping_count = ping_count + 1 WHERE id = 1 RETURNING ping_count",
				);
				websocket.send(
					JSON.stringify({
						type: "pong",
						pingCount: counter.ping_count,
						timestamp: Date.now(),
					}),
				);
			}
		});
	},
	actions: {
		async getPingCount(c) {
			const [counter] = await c.db.execute<{ ping_count: number }>(
				"SELECT ping_count FROM ping_counter WHERE id = 1",
			);
			return counter.ping_count;
		},
		async resetPingCount(c) {
			const [counter] = await c.db.execute<{ ping_count: number }>(
				"UPDATE ping_counter SET ping_count = 0 WHERE id = 1 RETURNING ping_count",
			);
			return counter.ping_count;
		},
	},
});
