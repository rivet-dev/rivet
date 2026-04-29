import { actor, type RivetMessageEvent, type UniversalWebSocket } from "rivetkit";

function sleep(ms: number): Promise<void> {
	return new Promise((resolve) => setTimeout(resolve, ms));
}

export const rawWebSocketServerlessSmoke = actor({
	options: {
		canHibernateWebSocket: false,
		sleepGracePeriod: 5_000,
	},
	state: {
		connectionCount: 0,
		sleepCount: 0,
		totalTickCount: 0,
		totalMessageCount: 0,
	},
	async onSleep(c) {
		const delayMs = 10 + Math.floor(Math.random() * 1_991);
		c.state.sleepCount += 1;
		c.log.info({
			msg: "raw websocket serverless smoke onSleep delay",
			delayMs,
			sleepCount: c.state.sleepCount,
		});
		await sleep(delayMs);
	},
	onWebSocket(c, websocket: UniversalWebSocket) {
		c.state.connectionCount += 1;
		const connectionId = crypto.randomUUID();
		let index = 0;

		const sendTick = () => {
			if (websocket.readyState !== 1) return;

			const timestamp = Date.now();
			const message = {
				type: "tick",
				connectionId,
				index,
				timestamp,
				iso: new Date(timestamp).toISOString(),
				totalTickCount: c.state.totalTickCount,
			};

			c.state.totalTickCount += 1;
			index += 1;
			websocket.send(JSON.stringify(message));
		};

		c.log.info({
			msg: "raw websocket serverless smoke connected",
			connectionId,
			connectionCount: c.state.connectionCount,
		});

		sendTick();
		const interval = setInterval(sendTick, 1_000);

		websocket.addEventListener("message", (event: RivetMessageEvent) => {
			c.state.totalMessageCount += 1;
			c.log.info({
				msg: "raw websocket serverless smoke received message",
				connectionId,
				totalMessageCount: c.state.totalMessageCount,
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
		});

		websocket.addEventListener("close", () => {
			clearInterval(interval);
			c.state.connectionCount -= 1;
			c.log.info({
				msg: "raw websocket serverless smoke disconnected",
				connectionId,
				connectionCount: c.state.connectionCount,
			});
		});
	},
	actions: {
		getStats(c) {
			return {
				connectionCount: c.state.connectionCount,
				sleepCount: c.state.sleepCount,
				totalTickCount: c.state.totalTickCount,
				totalMessageCount: c.state.totalMessageCount,
			};
		},
	},
});
