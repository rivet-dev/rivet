import { actor, event, type RivetMessageEvent, type UniversalWebSocket } from "rivetkit";

export const counter = actor({
	options: {
		canHibernateWebSocket: false,
		sleepGracePeriod: 5_000,
	},
	state: { count: 0 },
	events: {
		newCount: event<number>(),
	},
	onWebSocket(_c, websocket: UniversalWebSocket) {
		// Plain echo for the rtt counter-latency harness. Any message in →
		// the same payload back out. No state mutation, no awaits — keeps the
		// echo path as close to raw WS RTT as possible.
		websocket.addEventListener("message", (event: RivetMessageEvent) => {
			if (websocket.readyState !== 1) return;
			websocket.send(event.data as string | ArrayBuffer);
		});
	},
	actions: {
		increment: (c, x: number) => {
			c.state.count += x;
			c.broadcast("newCount", c.state.count);
			return c.state.count;
		},
		setCount: (c, x: number) => {
			c.state.count = x;
			c.broadcast("newCount", x);
			return c.state.count;
		},
		getCount: (c) => {
			return c.state.count;
		},
		noop: (_c) => {
			return { ok: true };
		},
		goToSleep: (c) => {
			c.sleep();
			return { ok: true };
		},
	},
});
