import {
	actor,
	type RivetMessageEvent,
	type UniversalWebSocket,
} from "rivetkit";
import { db } from "rivetkit/db";

export const tunnelStress = actor({
	options: {
		canHibernateWebSocket: false,
		sleepGracePeriod: 5_000,
	},
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS message_counter (
					id INTEGER PRIMARY KEY CHECK (id = 1),
					count INTEGER NOT NULL
				);
			`);
			await db.execute(
				"INSERT OR IGNORE INTO message_counter (id, count) VALUES (1, 0)",
			);
		},
	}),
	// Connection and heartbeat counts only describe open WebSockets, so they
	// live in memory. The message count is stored in SQLite.
	createVars: () => ({
		connectionCount: 0,
		heartbeatCount: 0,
	}),
	onWebSocket(c, websocket: UniversalWebSocket) {
		c.vars.connectionCount += 1;
		const connectionId = crypto.randomUUID();

		const sendHeartbeat = () => {
			if (websocket.readyState !== 1) return;

			c.vars.heartbeatCount += 1;
			websocket.send(
				JSON.stringify({
					type: "heartbeat",
					connectionId,
					heartbeatCount: c.vars.heartbeatCount,
					timestamp: Date.now(),
				}),
			);
		};

		const heartbeat = setInterval(sendHeartbeat, 1_000);
		sendHeartbeat();

		websocket.addEventListener(
			"message",
			async (event: RivetMessageEvent) => {
				// Fast-path ping: echo back without touching storage so the client can measure raw RTT
				// without the per-message storage write. Used by the counter-latency client's first
				// two probes after WS open.
				if (typeof event.data === "string") {
					let parsed: unknown;
					try {
						parsed = JSON.parse(event.data);
					} catch {
						parsed = undefined;
					}
					if (
						parsed &&
						typeof parsed === "object" &&
						(parsed as { type?: unknown }).type === "ping"
					) {
						const id = (parsed as { id?: unknown }).id;
						if (websocket.readyState === 1) {
							websocket.send(
								JSON.stringify({
									type: "pong",
									connectionId,
									id,
									timestamp: Date.now(),
								}),
							);
						}
						return;
					}
				}

				const [counter] = await c.db.execute<{ count: number }>(
					"UPDATE message_counter SET count = count + 1 WHERE id = 1 RETURNING count",
				);
				websocket.send(
					JSON.stringify({
						type: "reply",
						connectionId,
						messageCount: counter.count,
						timestamp: Date.now(),
						received: event.data,
					}),
				);
			},
		);

		websocket.addEventListener("close", () => {
			clearInterval(heartbeat);
			c.vars.connectionCount -= 1;
		});
	},
	actions: {
		async getStats(c) {
			const [counter] = await c.db.execute<{ count: number }>(
				"SELECT count FROM message_counter WHERE id = 1",
			);
			return {
				connectionCount: c.vars.connectionCount,
				messageCount: counter.count,
				heartbeatCount: c.vars.heartbeatCount,
			};
		},
	},
});
