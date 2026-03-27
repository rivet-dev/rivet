import { actor, event, type UniversalWebSocket } from "rivetkit";
import { promiseWithResolvers } from "rivetkit/utils";

export const SLEEP_TIMEOUT = 1000;

export const sleep = actor({
	state: { startCount: 0, sleepCount: 0 },
	onWake: (c) => {
		c.state.startCount += 1;
	},
	onSleep: (c) => {
		c.state.sleepCount += 1;
	},
	actions: {
		triggerSleep: (c) => {
			c.sleep();
		},
		getCounts: (c) => {
			return {
				startCount: c.state.startCount,
				sleepCount: c.state.sleepCount,
			};
		},
		setAlarm: async (c, duration: number) => {
			await c.schedule.after(duration, "onAlarm");
		},
		onAlarm: (c) => {
			c.log.info("alarm called");
		},
	},
	options: {
		sleepTimeout: SLEEP_TIMEOUT,
	},
});

export const sleepWithLongRpc = actor({
	state: { startCount: 0, sleepCount: 0 },
	createVars: () =>
		({}) as { longRunningResolve: PromiseWithResolvers<void> },
	onWake: (c) => {
		c.state.startCount += 1;
	},
	onSleep: (c) => {
		c.state.sleepCount += 1;
	},
	actions: {
		getCounts: (c) => {
			return {
				startCount: c.state.startCount,
				sleepCount: c.state.sleepCount,
			};
		},
		longRunningRpc: async (c) => {
			c.log.info("starting long running rpc");
			c.vars.longRunningResolve = promiseWithResolvers((reason) =>
				c.log.warn({
					msg: "unhandled long running rpc rejection",
					reason,
				}),
			);
			c.broadcast("waiting");
			await c.vars.longRunningResolve.promise;
			c.log.info("finished long running rpc");
		},
		finishLongRunningRpc: (c) => c.vars.longRunningResolve?.resolve(),
	},
	options: {
		sleepTimeout: SLEEP_TIMEOUT,
	},
});

export const sleepWithWaitUntilMessage = actor({
	state: {
		startCount: 0,
		sleepCount: 0,
		waitUntilMessageCount: 0,
	},
	events: {
		sleeping: event<{ sleepCount: number; startCount: number }>(),
	},
	onWake: (c) => {
		c.state.startCount += 1;
	},
	onSleep: (c) => {
		c.state.sleepCount += 1;
	},
	actions: {
		triggerSleep: (c) => {
			c.waitUntil(
				new Promise<void>((resolve) => {
					setTimeout(() => {
						c.state.waitUntilMessageCount += 1;
						c.conn.send("sleeping", {
							sleepCount: c.state.sleepCount,
							startCount: c.state.startCount,
						});
						resolve();
					}, 10);
				}),
			);
			c.sleep();
		},
		getCounts: (c) => {
			return {
				startCount: c.state.startCount,
				sleepCount: c.state.sleepCount,
				waitUntilMessageCount: c.state.waitUntilMessageCount,
			};
		},
	},
	options: {
		sleepTimeout: SLEEP_TIMEOUT,
	},
});

export const sleepWithRawHttp = actor({
	state: { startCount: 0, sleepCount: 0, requestCount: 0 },
	onWake: (c) => {
		c.state.startCount += 1;
	},
	onSleep: (c) => {
		c.state.sleepCount += 1;
	},
	onRequest: async (c, request) => {
		c.state.requestCount += 1;
		const url = new URL(request.url);

		if (url.pathname === "/long-request") {
			const duration = parseInt(
				url.searchParams.get("duration") || "1000",
			);
			c.log.info({ msg: "starting long fetch request", duration });
			await new Promise((resolve) => setTimeout(resolve, duration));
			c.log.info("finished long fetch request");
			return new Response(JSON.stringify({ completed: true }), {
				headers: { "Content-Type": "application/json" },
			});
		}

		return new Response("Not Found", { status: 404 });
	},
	actions: {
		getCounts: (c) => {
			return {
				startCount: c.state.startCount,
				sleepCount: c.state.sleepCount,
				requestCount: c.state.requestCount,
			};
		},
	},
	options: {
		sleepTimeout: SLEEP_TIMEOUT,
	},
});

export const sleepWithRawWebSocket = actor({
	state: { startCount: 0, sleepCount: 0, connectionCount: 0 },
	onWake: (c) => {
		c.state.startCount += 1;
	},
	onSleep: (c) => {
		c.state.sleepCount += 1;
	},
	onWebSocket: (c, websocket: UniversalWebSocket) => {
		c.state.connectionCount += 1;
		c.log.info({
			msg: "websocket connected",
			connectionCount: c.state.connectionCount,
		});

		websocket.send(
			JSON.stringify({
				type: "connected",
				connectionCount: c.state.connectionCount,
			}),
		);

		websocket.addEventListener("message", (event: any) => {
			const data = event.data;
			if (typeof data === "string") {
				try {
					const parsed = JSON.parse(data);
					if (parsed.type === "getCounts") {
						websocket.send(
							JSON.stringify({
								type: "counts",
								startCount: c.state.startCount,
								sleepCount: c.state.sleepCount,
								connectionCount: c.state.connectionCount,
							}),
						);
					} else if (parsed.type === "keepAlive") {
						// Just acknowledge to keep connection alive
						websocket.send(JSON.stringify({ type: "ack" }));
					}
				} catch {
					// Echo non-JSON messages
					websocket.send(data);
				}
			}
		});

		websocket.addEventListener("close", () => {
			c.state.connectionCount -= 1;
			c.log.info({
				msg: "websocket disconnected",
				connectionCount: c.state.connectionCount,
			});
		});
	},
	actions: {
		getCounts: (c) => {
			return {
				startCount: c.state.startCount,
				sleepCount: c.state.sleepCount,
				connectionCount: c.state.connectionCount,
			};
		},
	},
	options: {
		sleepTimeout: SLEEP_TIMEOUT,
	},
});

export const sleepRawWsSendOnSleep = actor({
	state: { startCount: 0, sleepCount: 0 },
	createVars: () => ({
		websockets: [] as UniversalWebSocket[],
	}),
	onWake: (c) => {
		c.state.startCount += 1;
	},
	onSleep: (c) => {
		c.state.sleepCount += 1;
		for (const ws of c.vars.websockets) {
			ws.send(JSON.stringify({ type: "sleeping", sleepCount: c.state.sleepCount }));
		}
	},
	onWebSocket: (c, websocket: UniversalWebSocket) => {
		c.vars.websockets.push(websocket);

		websocket.send(JSON.stringify({ type: "connected" }));

		websocket.addEventListener("close", () => {
			c.vars.websockets = c.vars.websockets.filter((ws) => ws !== websocket);
		});
	},
	actions: {
		triggerSleep: (c) => {
			c.sleep();
		},
		getCounts: (c) => {
			return {
				startCount: c.state.startCount,
				sleepCount: c.state.sleepCount,
			};
		},
	},
	options: {
		sleepTimeout: SLEEP_TIMEOUT,
	},
});

export const sleepRawWsDelayedSendOnSleep = actor({
	state: { startCount: 0, sleepCount: 0 },
	createVars: () => ({
		websockets: [] as UniversalWebSocket[],
	}),
	onWake: (c) => {
		c.state.startCount += 1;
	},
	onSleep: async (c) => {
		c.state.sleepCount += 1;
		// Wait before sending
		await new Promise((resolve) => setTimeout(resolve, 100));
		for (const ws of c.vars.websockets) {
			ws.send(JSON.stringify({ type: "sleeping", sleepCount: c.state.sleepCount }));
		}
		// Wait after sending before completing sleep
		await new Promise((resolve) => setTimeout(resolve, 100));
	},
	onWebSocket: (c, websocket: UniversalWebSocket) => {
		c.vars.websockets.push(websocket);

		websocket.send(JSON.stringify({ type: "connected" }));

		websocket.addEventListener("close", () => {
			c.vars.websockets = c.vars.websockets.filter((ws) => ws !== websocket);
		});
	},
	actions: {
		triggerSleep: (c) => {
			c.sleep();
		},
		getCounts: (c) => {
			return {
				startCount: c.state.startCount,
				sleepCount: c.state.sleepCount,
			};
		},
	},
	options: {
		sleepTimeout: SLEEP_TIMEOUT,
	},
});

export const sleepWithNoSleepOption = actor({
	state: { startCount: 0, sleepCount: 0 },
	onWake: (c) => {
		c.state.startCount += 1;
	},
	onSleep: (c) => {
		c.state.sleepCount += 1;
	},
	actions: {
		getCounts: (c) => {
			return {
				startCount: c.state.startCount,
				sleepCount: c.state.sleepCount,
			};
		},
	},
	options: {
		sleepTimeout: SLEEP_TIMEOUT,
		noSleep: true,
	},
});

export const sleepWithPreventSleep = actor({
	state: {
		startCount: 0,
		sleepCount: 0,
		preventSleepOnWake: false,
	},
	onWake: (c) => {
		c.state.startCount += 1;
		c.setPreventSleep(c.state.preventSleepOnWake);
	},
	onSleep: (c) => {
		c.state.sleepCount += 1;
	},
	actions: {
		triggerSleep: (c) => {
			c.sleep();
		},
		getStatus: (c) => {
			return {
				startCount: c.state.startCount,
				sleepCount: c.state.sleepCount,
				preventSleep: c.preventSleep,
				preventSleepOnWake: c.state.preventSleepOnWake,
			};
		},
		setPreventSleep: (c, prevent: boolean) => {
			c.setPreventSleep(prevent);
			return c.preventSleep;
		},
		setPreventSleepOnWake: (c, prevent: boolean) => {
			c.state.preventSleepOnWake = prevent;
			return c.state.preventSleepOnWake;
		},
	},
	options: {
		sleepTimeout: SLEEP_TIMEOUT,
	},
});
