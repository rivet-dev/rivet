import { actor, type RivetMessageEvent, type UniversalWebSocket } from "rivetkit";

// Minimal non-hibernatable WebSocket actor for fuzz-testing the
// force-sleep → gateway close path. Keeps state intentionally tiny so the
// only thing being exercised is the close lifecycle, not user code.
export const sleepCloseFuzz = actor({
	options: {
		canHibernateWebSocket: false,
	},
	state: {
		connectionCount: 0,
		messageCount: 0,
	},
	onWebSocket(c, websocket: UniversalWebSocket) {
		c.state.connectionCount += 1;
		const connectionId = crypto.randomUUID();

		websocket.send(
			JSON.stringify({
				type: "welcome",
				connectionId,
				connectionCount: c.state.connectionCount,
			}),
		);

		const interval = setInterval(() => {
			if (websocket.readyState !== 1) return;
			websocket.send(
				JSON.stringify({
					type: "tick",
					connectionId,
					timestamp: Date.now(),
				}),
			);
		}, 500);

		websocket.addEventListener("message", (event: RivetMessageEvent) => {
			c.state.messageCount += 1;
			websocket.send(
				JSON.stringify({
					type: "echo",
					connectionId,
					received: event.data,
				}),
			);
		});

		websocket.addEventListener("close", () => {
			clearInterval(interval);
			c.state.connectionCount -= 1;
		});
	},
	actions: {
		getStats(c) {
			return {
				connectionCount: c.state.connectionCount,
				messageCount: c.state.messageCount,
			};
		},
	},
});
