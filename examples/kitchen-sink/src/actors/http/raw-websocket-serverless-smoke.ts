import {
	actor,
	type RivetMessageEvent,
	type UniversalWebSocket,
} from "rivetkit";
import { db } from "rivetkit/db";

type SmokeCountsRow = {
	sleep_count: number;
	total_tick_count: number;
	total_message_count: number;
};

function sleep(ms: number): Promise<void> {
	return new Promise((resolve) => setTimeout(resolve, ms));
}

export const rawWebSocketServerlessSmoke = actor({
	options: {
		canHibernateWebSocket: false,
		sleepGracePeriod: 5_000,
	},
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS smoke_counts (
					id INTEGER PRIMARY KEY CHECK (id = 1),
					sleep_count INTEGER NOT NULL DEFAULT 0,
					total_tick_count INTEGER NOT NULL DEFAULT 0,
					total_message_count INTEGER NOT NULL DEFAULT 0
				);
			`);
			await db.execute(
				"INSERT OR IGNORE INTO smoke_counts (id) VALUES (1)",
			);
		},
	}),
	// Open WebSockets do not survive sleep, so the connection count lives in
	// memory. The totals are stored in SQLite.
	createVars: () => ({ connectionCount: 0 }),
	async onSleep(c) {
		const delayMs = 10 + Math.floor(Math.random() * 1_991);
		const [counts] = await c.db.execute<{ sleep_count: number }>(
			"UPDATE smoke_counts SET sleep_count = sleep_count + 1 WHERE id = 1 RETURNING sleep_count",
		);
		c.log.info({
			msg: "raw websocket serverless smoke onSleep delay",
			delayMs,
			sleepCount: counts.sleep_count,
		});
		await sleep(delayMs);
	},
	onWebSocket(c, websocket: UniversalWebSocket) {
		c.vars.connectionCount += 1;
		const connectionId = crypto.randomUUID();
		let index = 0;

		const sendTick = async () => {
			if (websocket.readyState !== 1) return;

			const [counts] = await c.db.execute<{ total_tick_count: number }>(
				"UPDATE smoke_counts SET total_tick_count = total_tick_count + 1 WHERE id = 1 RETURNING total_tick_count",
			);
			const timestamp = Date.now();
			const message = {
				type: "tick",
				connectionId,
				index,
				timestamp,
				iso: new Date(timestamp).toISOString(),
				// The total counts the ticks sent before this one.
				totalTickCount: counts.total_tick_count - 1,
			};

			index += 1;
			websocket.send(JSON.stringify(message));
		};

		const tick = () => {
			sendTick().catch((error) => {
				c.log.warn({
					msg: "raw websocket serverless smoke failed to send tick",
					connectionId,
					error,
				});
			});
		};

		c.log.info({
			msg: "raw websocket serverless smoke connected",
			connectionId,
			connectionCount: c.vars.connectionCount,
		});

		tick();
		const interval = setInterval(tick, 1_000);

		websocket.addEventListener(
			"message",
			async (event: RivetMessageEvent) => {
				const [counts] = await c.db.execute<{
					total_message_count: number;
				}>(
					"UPDATE smoke_counts SET total_message_count = total_message_count + 1 WHERE id = 1 RETURNING total_message_count",
				);
				c.log.info({
					msg: "raw websocket serverless smoke received message",
					connectionId,
					totalMessageCount: counts.total_message_count,
				});
				websocket.send(
					JSON.stringify({
						type: "ack",
						connectionId,
						index,
						timestamp: Date.now(),
						received: event.data,
					}),
				);
			},
		);

		websocket.addEventListener("close", () => {
			clearInterval(interval);
			c.vars.connectionCount -= 1;
			c.log.info({
				msg: "raw websocket serverless smoke disconnected",
				connectionId,
				connectionCount: c.vars.connectionCount,
			});
		});
	},
	actions: {
		async getStats(c) {
			const [counts] = await c.db.execute<SmokeCountsRow>(
				"SELECT sleep_count, total_tick_count, total_message_count FROM smoke_counts WHERE id = 1",
			);
			return {
				connectionCount: c.vars.connectionCount,
				sleepCount: counts.sleep_count,
				totalTickCount: counts.total_tick_count,
				totalMessageCount: counts.total_message_count,
			};
		},
	},
});
