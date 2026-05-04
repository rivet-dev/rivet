import { actor, event, type UniversalWebSocket } from "rivetkit";
import { promiseWithResolvers } from "rivetkit/utils";
import { scheduleActorSleep } from "./schedule-sleep";

export const SLEEP_TIMEOUT = 1000;
export const RAW_WS_HANDLER_SLEEP_TIMEOUT = 100;
export const RAW_WS_HANDLER_DELAY = 250;

type AsyncRawWebSocketState = {
	startCount: number;
	sleepCount: number;
	messageStarted: number;
	messageFinished: number;
	closeStarted: number;
	closeFinished: number;
};

function delay(ms: number) {
	return new Promise<void>((resolve) => setTimeout(resolve, ms));
}

function createAsyncRawWebSocketSleepActor(
	registration: "listener" | "property",
	eventType: "message" | "close",
) {
	return actor({
		state: {
			startCount: 0,
			sleepCount: 0,
			messageStarted: 0,
			messageFinished: 0,
			closeStarted: 0,
			closeFinished: 0,
		} satisfies AsyncRawWebSocketState,
		createVars: () => ({
			websocket: null as UniversalWebSocket | null,
		}),
		onWake: (c) => {
			c.state.startCount += 1;
		},
		onSleep: (c) => {
			c.state.sleepCount += 1;
		},
		onWebSocket: (c, websocket: UniversalWebSocket) => {
			c.vars.websocket = websocket;

			const onMessage = async (event: any) => {
				if (event.data !== "track-message") return;

				c.state.messageStarted += 1;
				websocket.send(JSON.stringify({ type: "message-started" }));
				await delay(RAW_WS_HANDLER_DELAY);
				c.state.messageFinished += 1;
			};

			const onClose = async () => {
				c.state.closeStarted += 1;
				await delay(RAW_WS_HANDLER_DELAY);
				c.state.closeFinished += 1;
			};

			if (registration === "listener") {
				if (eventType === "message") {
					websocket.addEventListener("message", onMessage);
				} else {
					websocket.addEventListener("close", onClose);
				}
			} else if (eventType === "message") {
				websocket.onmessage = onMessage;
			} else {
				websocket.onclose = onClose;
			}

			websocket.send(JSON.stringify({ type: "connected" }));
		},
		actions: {
			triggerSleep: (c) => {
				c.sleep();
			},
			getStatus: (c) => {
				return {
					startCount: c.state.startCount,
					sleepCount: c.state.sleepCount,
					messageStarted: c.state.messageStarted,
					messageFinished: c.state.messageFinished,
					closeStarted: c.state.closeStarted,
					closeFinished: c.state.closeFinished,
				};
			},
		},
		options: {
			sleepTimeout: RAW_WS_HANDLER_SLEEP_TIMEOUT,
		},
	});
}

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
			scheduleActorSleep(c);
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

export const sleepRawWsAddEventListenerMessage =
	createAsyncRawWebSocketSleepActor("listener", "message");

export const sleepRawWsAddEventListenerClose =
	createAsyncRawWebSocketSleepActor("listener", "close");

export const sleepRawWsOnMessage = createAsyncRawWebSocketSleepActor(
	"property",
	"message",
);

export const sleepRawWsOnClose = createAsyncRawWebSocketSleepActor(
	"property",
	"close",
);

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
			ws.send(
				JSON.stringify({
					type: "sleeping",
					sleepCount: c.state.sleepCount,
				}),
			);
		}
	},
	onWebSocket: (c, websocket: UniversalWebSocket) => {
		c.vars.websockets.push(websocket);

		websocket.send(JSON.stringify({ type: "connected" }));

		websocket.addEventListener("close", () => {
			c.vars.websockets = c.vars.websockets.filter(
				(ws) => ws !== websocket,
			);
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
			ws.send(
				JSON.stringify({
					type: "sleeping",
					sleepCount: c.state.sleepCount,
				}),
			);
		}
		// Wait after sending before completing sleep
		await new Promise((resolve) => setTimeout(resolve, 100));
	},
	onWebSocket: (c, websocket: UniversalWebSocket) => {
		c.vars.websockets.push(websocket);

		websocket.send(JSON.stringify({ type: "connected" }));

		websocket.addEventListener("close", () => {
			c.vars.websockets = c.vars.websockets.filter(
				(ws) => ws !== websocket,
			);
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

export const sleepWithWaitUntilInOnWake = actor({
	state: {
		startCount: 0,
		sleepCount: 0,
		waitUntilCalled: false,
		waitUntilCompleted: false,
	},
	onWake: (c) => {
		c.state.startCount += 1;
		// This should not throw. Before the fix, assertReady() would throw
		// because #ready is false during onWake.
		c.waitUntil(
			(async () => {
				c.state.waitUntilCompleted = true;
			})(),
		);
		c.state.waitUntilCalled = true;
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
				waitUntilCalled: c.state.waitUntilCalled,
				waitUntilCompleted: c.state.waitUntilCompleted,
			};
		},
	},
	options: {
		sleepTimeout: SLEEP_TIMEOUT,
	},
});

export const counterWaitUntilProbe = actor({
	state: { count: 0 },
	actions: {
		triggerWaitUntilVoid: (c) => {
			c.state.count += 1;
			c.waitUntil(Promise.resolve());
			return c.state.count;
		},
		triggerWaitUntilWithValue: (c) => {
			c.state.count += 1;
			c.waitUntil(Promise.resolve({ ok: true }));
			return c.state.count;
		},
		triggerWaitUntilRejectVoid: (c) => {
			c.state.count += 1;
			c.waitUntil(Promise.reject(new Error("reject-with-error-ok")));
			return c.state.count;
		},
		triggerKeepAwakeVoid: async (c) => {
			c.state.count += 1;
			await c.keepAwake(Promise.resolve());
			return c.state.count;
		},
		triggerKeepAwakeWithValue: async (c) => {
			c.state.count += 1;
			await c.keepAwake(Promise.resolve({ ok: true }));
			return c.state.count;
		},
		getCount: (c) => c.state.count,
	},
	options: {
		sleepTimeout: SLEEP_TIMEOUT,
	},
});

type AbortVarsObservation =
	| "unset"
	| "object"
	| "undefined"
	| `error:${string}`;

export const sleepAbortListenerVarsActor = actor({
	state: {
		startCount: 0,
		sleepCount: 0,
		// Per-wake observations keyed by `startCount` (1-indexed). Each
		// wake registers its own abort listener that records what
		// `c.vars` looked like when the listener fired. The user-reported
		// crash typically lands on a *later* wake/sleep cycle once the
		// runtime has been bounced through reset/cleanup at least once.
		abortVarsSeenPerWake: [] as AbortVarsObservation[],
	},
	createVars: () => ({
		isStopping: false,
	}),
	onWake: (c) => {
		c.state.startCount += 1;
		const wakeIndex = c.state.startCount;
		c.vars.isStopping = c.aborted;

		// Reserve a slot for this wake's observation.
		const observations = [...c.state.abortVarsSeenPerWake];
		observations[wakeIndex - 1] = "unset";
		c.state.abortVarsSeenPerWake = observations;

		if (c.aborted) {
			return;
		}

		c.abortSignal.addEventListener(
			"abort",
			() => {
				let observation: AbortVarsObservation;
				try {
					const vars = c.vars as
						| { isStopping: boolean }
						| undefined;
					if (vars === undefined || vars === null) {
						observation = "undefined";
					} else {
						vars.isStopping = true;
						observation = "object";
					}
				} catch (error) {
					observation = `error:${
						error instanceof Error
							? error.message
							: String(error)
					}`;
				}
				try {
					const next = [...c.state.abortVarsSeenPerWake];
					next[wakeIndex - 1] = observation;
					c.state.abortVarsSeenPerWake = next;
				} catch {
					// State write may itself fail if the bag was cleared.
					// Swallow so the test still reads `getStatus` cleanly.
				}
			},
			{ once: true },
		);
	},
	onSleep: async (c) => {
		c.state.sleepCount += 1;
		// Drain a few macrotasks so the cleanup that runs in this handler's
		// `finally` (cleanupNativeSleepRuntimeState) has the best chance of
		// landing before any other shutdown TSF (notably the abort signal)
		// is processed on the JS side. This makes the abort-listener race
		// observable in the test.
		for (let i = 0; i < 5; i += 1) {
			await new Promise<void>((resolve) => setTimeout(resolve, 0));
		}
	},
	onWebSocket: (c, websocket) => {
		websocket.send(JSON.stringify({ type: "connected" }));
	},
	actions: {
		triggerSleep: (c) => {
			c.sleep();
		},
		getStatus: (c) => {
			return {
				startCount: c.state.startCount,
				sleepCount: c.state.sleepCount,
				abortVarsSeenPerWake: c.state.abortVarsSeenPerWake,
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
