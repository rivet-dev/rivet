import * as protocol from "@rivetkit/engine-envoy-protocol";
import type { UnboundedSender } from "antiox/sync/mpsc";
import { sleep } from "antiox/time";
import { JoinHandle, spawn } from "antiox/task";
import type { SharedContext } from "../context.js";
import { logger } from "../log.js";
import { stringifyToEnvoy, stringifyToRivet } from "../stringify.js";
import { calculateBackoff, ParsedCloseReason, parseWebSocketCloseReason } from "../utils.js";
import {
	type WebSocketRxMessage,
	type WebSocketTxMessage,
	webSocket,
} from "../websocket.js";

export function startConnection(ctx: SharedContext): JoinHandle<void> {
	return spawn(signal => connectionLoop(ctx, signal));
}

const STABLE_CONNECTION_MS = 60_000;

async function connectionLoop(ctx: SharedContext, signal: AbortSignal) {
	let attempt = 0;
	while (true) {
		const connectedAt = Date.now();
		try {
			const res = await singleConnection(ctx, signal);

			if (res) {
				if (res.group === "ws" && res.error === "eviction") {
					log(ctx)?.debug({
						msg: "connection evicted",
					});

					ctx.envoyTx.send({ type: "conn-close", evict: true });

					return;
				} else if (res.group === 'channel' && res.error === "closed") {
					// Client side shutdown
					return;
				}
			}

			ctx.envoyTx.send({ type: "conn-close", evict: false });
		} catch (error) {
			log(ctx)?.error({
				msg: "connection failed",
				error,
			});

			ctx.envoyTx.send({ type: "conn-close", evict: false });
		}

		if (Date.now() - connectedAt >= STABLE_CONNECTION_MS) {
			attempt = 0;
		}

		const delay = calculateBackoff(attempt);
		log(ctx)?.info({
			msg: "reconnecting",
			attempt,
			delayMs: delay,
		});
		await sleep(delay);
		attempt++;
	}
}

async function singleConnection(ctx: SharedContext, signal: AbortSignal): Promise<ParsedCloseReason | undefined> {
	const { config } = ctx;

	const protocols = ["rivet"];
	if (config.token) protocols.push(`rivet_token.${config.token}`);

	const [wsTx, wsRx] = await webSocket({
		url: wsUrl(ctx),
		protocols,
		debugLatencyMs: config.debugLatencyMs,
	});
	ctx.wsTx = wsTx;

	log(ctx)?.info({
		msg: "websocket connected",
		endpoint: config.endpoint,
		namespace: config.namespace,
		envoyKey: ctx.envoyKey,
		hasToken: !!config.token,
	});

	wsSend(ctx, {
		tag: "ToRivetInit",
		val: {
			envoyKey: ctx.envoyKey,
			version: config.version,
			prepopulateActorNames: new Map(
				Object.entries(config.prepopulateActorNames).map(
					([name, data]) => [
						name,
						{ metadata: JSON.stringify(data.metadata) },
					],
				),
			),
			metadata: JSON.stringify(config.metadata),
		},
	});

	let res;

	try {
		for await (const msg of wsRx) {
			if (msg.type === "message") {
				await handleWsData(ctx, msg);
			} else if (msg.type === "close") {
				log(ctx)?.info({
					msg: "websocket closed",
					code: msg.code,
					reason: msg.reason,
				});
				res = parseWebSocketCloseReason(msg.reason);
				break;
			} else if (msg.type === "error") {
				log(ctx)?.error({
					msg: "websocket error",
					error: msg.error,
				});
				break;
			}
		}

		res = { group: "channel", error: "closed" };
	} finally {
		ctx.wsTx = undefined;
	}

	return res;
}

async function handleWsData(
	ctx: SharedContext,
	msg: WebSocketRxMessage & { type: "message" },
) {
	let buf: Uint8Array;
	if (msg.data instanceof Blob) {
		buf = new Uint8Array(await msg.data.arrayBuffer());
	} else if (Buffer.isBuffer(msg.data)) {
		buf = new Uint8Array(msg.data);
	} else if (msg.data instanceof ArrayBuffer) {
		buf = new Uint8Array(msg.data);
	} else {
		throw new Error(`expected binary data, got ${typeof msg.data}`);
	}

	const message = protocol.decodeToEnvoy(buf);
	log(ctx)?.debug({
		msg: "received message",
		data: stringifyToEnvoy(message),
	});

	forwardToEnvoy(ctx, message);
}

function forwardToEnvoy(ctx: SharedContext, message: protocol.ToEnvoy) {
	if (message.tag === "ToEnvoyPing") {
		wsSend(ctx, {
			tag: "ToRivetPong",
			val: { ts: message.val.ts },
		});
	} else {
		if (ctx.envoyTx.isClosed()) console.error("envoy tx should not be closed");

		ctx.envoyTx.send({ type: "conn-message", message });
	}
}

// Returns true if not sent.
export function wsSend(ctx: SharedContext, message: protocol.ToRivet): boolean {
	log(ctx)?.debug({
		msg: "sending message",
		data: stringifyToRivet(message),
	});

	// We don't queue messages when the ws isn't available because any durable messages we need to send are
	// tracked via either the event history or the buffered tunnel messages system
	if (!ctx.wsTx) {
		log(ctx)?.error({
			msg: "websocket not available for sending",
		});
		return true;
	}

	const encoded = protocol.encodeToRivet(message);
	ctx.wsTx.send({ type: "send", data: encoded });

	return false;
}

function wsUrl(ctx: SharedContext) {
	const wsEndpoint = ctx.config.endpoint
		.replace("http://", "ws://")
		.replace("https://", "wss://");

	const baseUrl = wsEndpoint.endsWith("/")
		? wsEndpoint.slice(0, -1)
		: wsEndpoint;
	const parameters = [
		["protocol_version", protocol.VERSION],
		["namespace", ctx.config.namespace],
		["envoy_key", ctx.envoyKey],
		["pool_name", ctx.config.poolName],
	];

	return `${baseUrl}/envoys/connect?${parameters
		.map(([key, value]) => `${key}=${encodeURIComponent(value)}`)
		.join("&")}`;
}

function log(ctx: SharedContext) {
	if (ctx.logCached) return ctx.logCached;

	const baseLogger = ctx.config.logger ?? logger();
	if (!baseLogger) return undefined;

	ctx.logCached = baseLogger.child({
		envoyKey: ctx.envoyKey,
	});
	return ctx.logCached;
}
