import { actor, type RivetMessageEvent, type UniversalWebSocket } from "rivetkit";

export const tunnelStress = actor({
	options: {
		canHibernateWebSocket: false,
		sleepGracePeriod: 5_000,
	},
	state: {
		connectionCount: 0,
		messageCount: 0,
		heartbeatCount: 0,
	},
	onWebSocket(c, websocket: UniversalWebSocket) {
		c.state.connectionCount += 1;
		const connectionId = crypto.randomUUID();

		const sendHeartbeat = () => {
			if (websocket.readyState !== 1) return;

			c.state.heartbeatCount += 1;
			websocket.send(
				JSON.stringify({
					type: "heartbeat",
					connectionId,
					heartbeatCount: c.state.heartbeatCount,
					timestamp: Date.now(),
				}),
			);
		};

		const heartbeat = setInterval(sendHeartbeat, 1_000);
		sendHeartbeat();

		websocket.addEventListener("message", async (event: RivetMessageEvent) => {
			c.state.messageCount += 1;
			await c.kv.put("counter", String(c.state.messageCount));
			websocket.send(
				JSON.stringify({
					type: "reply",
					connectionId,
					messageCount: c.state.messageCount,
					timestamp: Date.now(),
					received: event.data,
				}),
			);
		});

		websocket.addEventListener("close", () => {
			clearInterval(heartbeat);
			c.state.connectionCount -= 1;
		});
	},
	actions: {
		getStats(c) {
			return {
				connectionCount: c.state.connectionCount,
				messageCount: c.state.messageCount,
				heartbeatCount: c.state.heartbeatCount,
			};
		},
	},
});
