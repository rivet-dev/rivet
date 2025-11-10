import { actor } from "rivetkit";

type ConnParams = { trackLifecycle?: boolean } | undefined;

export const counterWithLifecycle = actor({
	state: {
		count: 0,
		events: [] as string[],
	},
	createConnState: (c, params: ConnParams) => ({
		joinTime: Date.now(),
	}),
	onWake: (c) => {
		c.state.events.push("onWake");
	},
	onBeforeConnect: (c, params: ConnParams) => {
		if (params?.trackLifecycle) c.state.events.push("onBeforeConnect");
	},
	onConnect: (c, conn) => {
		if (conn.params?.trackLifecycle) c.state.events.push("onConnect");
	},
	onDisconnect: (c, conn) => {
		if (conn.params?.trackLifecycle) c.state.events.push("onDisconnect");
	},
	actions: {
		getEvents: (c) => {
			return c.state.events;
		},
		increment: (c, x: number) => {
			c.state.count += x;
			return c.state.count;
		},
	},
});
