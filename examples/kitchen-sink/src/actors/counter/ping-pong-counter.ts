import { actor } from "rivetkit";

export const pingPongCounter = actor({
	state: {
		pingCount: 0,
	},
	onWebSocket(ctx, websocket) {
		websocket.addEventListener("message", (event: any) => {
			const data = event.data;
			if (typeof data !== "string") return;

			let parsed: any;
			try {
				parsed = JSON.parse(data);
			} catch {
				return;
			}

			if (parsed?.type === "ping") {
				ctx.state.pingCount = ctx.state.pingCount + 1;
				websocket.send(
					JSON.stringify({
						type: "pong",
						pingCount: ctx.state.pingCount,
						timestamp: Date.now(),
					}),
				);
			}
		});
	},
	actions: {
		getPingCount(c) {
			return c.state.pingCount;
		},
		resetPingCount(c) {
			c.state.pingCount = 0;
			return c.state.pingCount;
		},
	},
});
