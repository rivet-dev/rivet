import { type ActorContext, actor, type UniversalWebSocket } from "rivetkit";
import { scheduleActorSleep } from "./schedule-sleep";

export const rawWebSocketActor = actor({
	options: {
		canHibernateWebSocket: true,
		sleepTimeout: 250,
	},
	state: {
		connectionCount: 0,
		messageCount: 0,
		indexedMessageOrder: [] as number[],
	},
	onWebSocket(ctx, websocket) {
		ctx.state.connectionCount = ctx.state.connectionCount + 1;
		console.log(
			`[ACTOR] New connection, count: ${ctx.state.connectionCount}`,
		);

		// Send welcome message
		websocket.send(
			JSON.stringify({
				type: "welcome",
				connectionCount: ctx.state.connectionCount,
			}),
		);
		console.log("[ACTOR] Sent welcome message");

		// Echo messages back
		websocket.addEventListener("message", (event: any) => {
			ctx.state.messageCount = ctx.state.messageCount + 1;
			console.log(
				`[ACTOR] Message received, total count: ${ctx.state.messageCount}, data:`,
				event.data,
			);

			const data = event.data;
			if (typeof data === "string") {
				try {
					const parsed = JSON.parse(data);
					if (parsed.type === "ping") {
						websocket.send(
							JSON.stringify({
								type: "pong",
								timestamp: Date.now(),
							}),
						);
					} else if (parsed.type === "getStats") {
						console.log(
							`[ACTOR] Sending stats - connections: ${ctx.state.connectionCount}, messages: ${ctx.state.messageCount}`,
						);
						websocket.send(
							JSON.stringify({
								type: "stats",
								connectionCount: ctx.state.connectionCount,
								messageCount: ctx.state.messageCount,
							}),
						);
					} else if (parsed.type === "getRequestInfo") {
						// Send back the request URL info if available
						const url = ctx.request?.url || "ws://actor/websocket";
						const urlObj = new URL(url);
						websocket.send(
							JSON.stringify({
								type: "requestInfo",
								url: url,
								pathname: urlObj.pathname,
								search: urlObj.search,
							}),
						);
					} else if (parsed.type === "indexedEcho") {
						const rivetMessageIndex =
							typeof event.rivetMessageIndex === "number"
								? event.rivetMessageIndex
								: null;
						ctx.state.indexedMessageOrder.push(rivetMessageIndex);
						websocket.send(
							JSON.stringify({
								type: "indexedEcho",
								payload: parsed.payload ?? null,
								rivetMessageIndex,
							}),
						);
					} else if (parsed.type === "indexedAckProbe") {
						const rivetMessageIndex =
							typeof event.rivetMessageIndex === "number"
								? event.rivetMessageIndex
								: null;
						ctx.state.indexedMessageOrder.push(rivetMessageIndex);
						websocket.send(
							JSON.stringify({
								type: "indexedAckProbe",
								rivetMessageIndex,
								payloadSize:
									typeof parsed.payload === "string"
										? parsed.payload.length
										: 0,
							}),
						);
					} else if (parsed.type === "getIndexedMessageOrder") {
						websocket.send(
							JSON.stringify({
								type: "indexedMessageOrder",
								order: ctx.state.indexedMessageOrder,
							}),
						);
					} else if (parsed.type === "scheduleSleep") {
						websocket.send(
							JSON.stringify({
								type: "sleepScheduled",
							}),
						);
						globalThis.setTimeout(() => {
							ctx.sleep();
						}, 25);
					} else {
						// Echo back
						websocket.send(data);
					}
				} catch {
					// If not JSON, just echo it back
					websocket.send(data);
				}
			} else {
				// Echo binary data
				websocket.send(data);
			}
		});

		// Handle close
		websocket.addEventListener("close", () => {
			ctx.state.connectionCount = ctx.state.connectionCount - 1;
			console.log(
				`[ACTOR] Connection closed, count: ${ctx.state.connectionCount}`,
			);
		});
	},
	actions: {
		triggerSleep: (c: ActorContext<any, any, any, any, any, any>) => {
			scheduleActorSleep(c);
			return true;
		},
		getStats(ctx: any) {
			return {
				connectionCount: ctx.state.connectionCount,
				messageCount: ctx.state.messageCount,
			};
		},
	},
});

export const rawWebSocketBinaryActor = actor({
	onWebSocket(ctx, websocket) {
		// Handle binary data
		websocket.addEventListener("message", (event: any) => {
			const data = event.data;
			if (data instanceof ArrayBuffer || data instanceof Uint8Array) {
				// Reverse the bytes and send back
				const bytes = new Uint8Array(data);
				const reversed = new Uint8Array(bytes.length);
				for (let i = 0; i < bytes.length; i++) {
					reversed[i] = bytes[bytes.length - 1 - i];
				}
				websocket.send(reversed);
			}
		});
	},
	actions: {},
});

export const rawWebSocketAsyncOpenActor = actor({
	options: {
		canHibernateWebSocket: false,
		sleepGracePeriod: 2_000,
	},
	state: {
		openCount: 0,
	},
	async onWebSocket(ctx, websocket) {
		ctx.state.openCount += 1;
		await new Promise((resolve) => setTimeout(resolve, 10));
		websocket.send(
			JSON.stringify({
				type: "async-open",
				openCount: ctx.state.openCount,
			}),
		);
	},
	actions: {
		getOpenCount: (ctx) => ctx.state.openCount,
	},
});

export const rawWebSocketConnContextActor = actor({
	onWebSocket(ctx: any, websocket: UniversalWebSocket) {
		const connId = ctx.conn.id;
		ctx.conn.state = {
			opened: true,
			connId,
		};
		websocket.send(
			JSON.stringify({
				type: "conn-context",
				connId,
				state: ctx.conn.state,
			}),
		);
	},
	actions: {},
});
