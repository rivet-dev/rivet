import { actor } from "rivetkit";
import { handleHttpRequest, httpActions } from "./http";
import { handleWebSocket, websocketActions } from "./websocket";

export const demo = actor({
	createState: (_c, input) => ({
		input,
		count: 0,
		lastMessage: "",
		alarmHistory: [] as { id: string; time: number; data?: any }[],
		startCount: 0,
		stopCount: 0,
	}),
	connState: {
		connectionTime: 0,
	},
	onWake: (c) => {
		c.state.startCount += 1;
		c.log.info({
			msg: "demo actor started",
			startCount: c.state.startCount,
		});
	},
	onSleep: (c) => {
		c.state.stopCount += 1;
		c.log.info({ msg: "demo actor stopped", stopCount: c.state.stopCount });
	},
	onConnect: (c, conn) => {
		conn.state.connectionTime = Date.now();
		c.log.info({
			msg: "client connected",
			connectionTime: conn.state.connectionTime,
		});
	},
	onDisconnect: (c) => {
		c.log.info("client disconnected");
	},
	onFetch: handleHttpRequest,
	onWebSocket: handleWebSocket,
	actions: {
		// Sync actions
		increment: (c, amount: number = 1) => {
			c.state.count += amount;
			c.broadcast("countChanged", { count: c.state.count, amount });
			return c.state.count;
		},
		getCount: (c) => {
			return c.state.count;
		},
		setMessage: (c, message: string) => {
			c.state.lastMessage = message;
			c.broadcast("messageChanged", { message });
			return message;
		},

		// Async actions
		delayedIncrement: async (
			c,
			amount: number = 1,
			delayMs: number = 1000,
		) => {
			await new Promise((resolve) => setTimeout(resolve, delayMs));
			c.state.count += amount;
			c.broadcast("countChanged", { count: c.state.count, amount });
			return c.state.count;
		},

		// Promise action
		promiseAction: () => {
			return Promise.resolve({
				timestamp: Date.now(),
				message: "promise resolved",
			});
		},

		// State management
		getState: (c) => {
			return {
				actorState: c.state,
				connectionState: c.conn.state,
			};
		},

		// Scheduling
		scheduleAlarmAt: (c, timestamp: number, data?: any) => {
			const id = `alarm-${Date.now()}`;
			c.schedule.at(timestamp, "onAlarm", { id, data });
			return { id, scheduledFor: timestamp };
		},
		scheduleAlarmAfter: (c, delayMs: number, data?: any) => {
			const id = `alarm-${Date.now()}`;
			c.schedule.after(delayMs, "onAlarm", { id, data });
			return { id, scheduledFor: Date.now() + delayMs };
		},
		onAlarm: (c, payload: { id: string; data?: any }) => {
			const alarmEntry = { ...payload, time: Date.now() };
			c.state.alarmHistory.push(alarmEntry);
			c.broadcast("alarmTriggered", alarmEntry);
			c.log.info({ msg: "alarm triggered", ...alarmEntry });
		},
		getAlarmHistory: (c) => {
			return c.state.alarmHistory;
		},
		clearAlarmHistory: (c) => {
			c.state.alarmHistory = [];
			return true;
		},

		// Sleep
		triggerSleep: (c) => {
			c.sleep();
			return "sleep triggered";
		},

		// Lifecycle info
		getLifecycleInfo: (c) => {
			return {
				startCount: c.state.startCount,
				stopCount: c.state.stopCount,
			};
		},

		// Metadata
		getMetadata: (c) => {
			return {
				name: c.name,
			};
		},
		getInput: (c) => {
			return c.state.input;
		},
		getActorState: (c) => {
			return c.state;
		},
		getConnState: (c) => {
			return c.conn.state;
		},

		// Events
		broadcastCustomEvent: (c, eventName: string, data: any) => {
			c.broadcast(eventName, data);
			return { eventName, data, timestamp: Date.now() };
		},

		// Connections
		listConnections: (c) => {
			return Array.from(c.conns.values()).map((conn) => ({
				id: conn.id,
				connectedAt: conn.state.connectionTime,
			}));
		},

		// HTTP actions
		...httpActions,

		// WebSocket actions
		...websocketActions,
	},
	options: {
		sleepTimeout: 5_000,
	},
});
