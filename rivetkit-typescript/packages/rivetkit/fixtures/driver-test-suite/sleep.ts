import { actor, type UniversalWebSocket } from "rivetkit";
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
			c.vars.longRunningResolve = promiseWithResolvers();
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
	onWebSocket: (c, websocket: UniversalWebSocket, opts) => {
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
