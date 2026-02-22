import { actor } from "rivetkit";

export const fileSystemHibernationCleanupActor = actor({
	state: {
		wakeCount: 0,
		sleepCount: 0,
		disconnectWakeCounts: [] as number[],
	},
	createConnState: () => ({}),
	onWake: (c) => {
		c.state.wakeCount += 1;
	},
	onSleep: (c) => {
		c.state.sleepCount += 1;
	},
	onDisconnect: (c, conn) => {
		// Only track WebSocket connection cleanup. HTTP actions are ephemeral.
		if (conn.isHibernatable) {
			c.state.disconnectWakeCounts.push(c.state.wakeCount);
		}
	},
	actions: {
		ping: () => "pong",
		triggerSleep: (c) => {
			c.sleep();
		},
		getCounts: (c) => ({
			wakeCount: c.state.wakeCount,
			sleepCount: c.state.sleepCount,
		}),
		getDisconnectWakeCounts: (c) => c.state.disconnectWakeCounts,
	},
	options: {
		sleepTimeout: 500,
	},
});
