import type { UnboundedReceiver, UnboundedSender } from "antiox/sync/mpsc";
import { OnceCell } from "antiox/sync/once_cell";
import { spawn } from "antiox/task";
import type WsWebSocket from "ws";
import { latencyChannel } from "./latency-channel.js";
import { logger } from "./log.js";

export type WebSocketTxData = Parameters<WebSocket["send"]>[0];

export type WebSocketRxData = WsWebSocket.Data | Blob;

export type WebSocketTxMessage =
	| { type: "send"; data: WebSocketTxData }
	| { type: "close"; code?: number; reason?: string };

export type WebSocketRxMessage =
	| { type: "message"; data: WebSocketRxData }
	| { type: "close"; code: number; reason: string }
	| { type: "error"; error: Error };

export type WebSocketHandle = [
	UnboundedSender<WebSocketTxMessage>,
	UnboundedReceiver<WebSocketRxMessage>,
];

export interface WebSocketOptions {
	url: string;
	protocols?: string | string[];
	debugLatencyMs?: number;
}

const webSocketPromise = new OnceCell<typeof WebSocket>();

export async function importWebSocket(): Promise<typeof WebSocket> {
	return webSocketPromise.getOrInit(async () => {
		let _WebSocket: typeof WebSocket;

		if (typeof WebSocket !== "undefined") {
			// Native
			_WebSocket = WebSocket as unknown as typeof WebSocket;
			logger()?.debug({ msg: "using native websocket" });
		} else {
			// Node.js package
			try {
				const ws = await import("ws");
				_WebSocket = ws.default as unknown as typeof WebSocket;
				logger()?.debug({ msg: "using websocket from npm" });
			} catch {
				// WS not available
				_WebSocket = class MockWebSocket {
					constructor() {
						throw new Error(
							'WebSocket support requires installing the "ws" peer dependency.',
						);
					}
				} as unknown as typeof WebSocket;
				logger()?.debug({ msg: "using mock websocket" });
			}
		}

		return _WebSocket;
	});
}

export async function webSocket(
	options: WebSocketOptions,
): Promise<WebSocketHandle> {
	const { url, protocols, debugLatencyMs } = options;
	const WS = await importWebSocket();
	const raw = new WS(url, protocols);
	const [outboundTx, outboundRx] =
		latencyChannel<WebSocketTxMessage>(debugLatencyMs);
	const [inboundTx, inboundRx] =
		latencyChannel<WebSocketRxMessage>(debugLatencyMs);

	raw.addEventListener("message", (event) => {
		inboundTx.send({
			type: "message",
			data: event.data as WebSocketRxData,
		});
	});

	raw.addEventListener("close", (event) => {
		inboundTx.send({
			type: "close",
			code: event.code,
			reason: event.reason,
		});
		inboundTx.close();
		outboundRx.close();
	});

	raw.addEventListener("error", (event) => {
		const error =
			typeof event === "object" && event !== null && "error" in event
				? event.error
				: new Error("WebSocket error");
		inboundTx.send({
			type: "error",
			error: error instanceof Error ? error : new Error(String(error)),
		});
		inboundTx.close();
		outboundRx.close();
	});

	spawn(async () => {
		for await (const message of outboundRx) {
			if (message.type === "send") {
				raw.send(message.data);
			} else {
				raw.close(message.code, message.reason);
				break;
			}
		}

		if (raw.readyState === 0 || raw.readyState === 1) {
			raw.close();
		}
		inboundTx.close();
	});

	// Wait for socket ready or error
	await new Promise((res, rej) => {
		raw.addEventListener("open", res, { once: true });
		raw.addEventListener("close", () => rej(new Error("websocket closed")), { once: true });
		raw.addEventListener("error", (event) => rej(event.error), { once: true });
	});

	return [outboundTx, inboundRx];
}
