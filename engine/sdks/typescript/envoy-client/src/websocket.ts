import type { UnboundedReceiver, UnboundedSender } from "antiox/sync/mpsc";
import { OnceCell } from "antiox/sync/once_cell";
import { spawn } from "antiox/task";
import type WsWebSocket from "ws";
import { latencyChannel } from "./latency-channel.js";
import { logger } from "./log.js";
import { VirtualWebSocket, type UniversalWebSocket, type RivetMessageEvent } from "@rivetkit/virtual-websocket";
import { wrappingAddU16, wrappingLteU16, wrappingSubU16 } from "./utils";
import { SharedContext } from "./context.js";
import { log } from "./tasks/envoy/index.js";

export const HIBERNATABLE_SYMBOL = Symbol("hibernatable");

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

export class WebSocketTunnelAdapter {
	#readyState: 0 | 1 | 2 | 3 = 0;
	#binaryType: "nodebuffer" | "arraybuffer" | "blob" = "nodebuffer";
	#shared: SharedContext;
	#ws: VirtualWebSocket;
	#actorId: string;
	#requestId: string;
	#hibernatable: boolean;
	#messageIndex: number;
	#sendCallback: (data: ArrayBuffer | string, isBinary: boolean) => void;
	#closeCallback: (code?: number, reason?: string) => void;

	get [HIBERNATABLE_SYMBOL](): boolean {
		return this.#hibernatable;
	}

	constructor(
		ctx: SharedContext,
		actorId: string,
		requestId: string,
		messageIndex: number,
		hibernatable: boolean,
		isRestoringHibernatable: boolean,
		public readonly request: Request,
		sendCallback: (data: ArrayBuffer | string, isBinary: boolean) => void,
		closeCallback: (code?: number, reason?: string) => void,
	) {
		this.#shared = ctx;
		this.#actorId = actorId;
		this.#requestId = requestId;
		this.#hibernatable = hibernatable;
		this.#messageIndex = messageIndex;
		this.#sendCallback = sendCallback;
		this.#closeCallback = closeCallback;

		this.#ws = new VirtualWebSocket({
			getReadyState: () => this.#readyState,
			onSend: (data) => this.#handleSend(data),
			onClose: (code, reason) => this.#close(code, reason, true),
			onTerminate: () => this.#terminate(),
		});

		if (isRestoringHibernatable) {
			log(this.#shared)?.debug({
				msg: "setting WebSocket to OPEN state for restored connection",
				actorId: this.#actorId,
				requestId: this.#requestId,
			});
			this.#readyState = 1;
		}
	}

	get websocket(): UniversalWebSocket {
		return this.#ws;
	}

	#handleSend(data: string | ArrayBufferLike | Blob | ArrayBufferView): void {
		let isBinary = false;
		let messageData: string | ArrayBuffer;

		const maxPayloadSize = this.#shared.protocolMetadata?.maxResponsePayloadSize ?? Infinity;

		if (typeof data === "string") {
			const encoder = new TextEncoder();
			if (encoder.encode(data).byteLength > maxPayloadSize) {
				throw new Error("WebSocket message too large");
			}

			messageData = data;
		} else if (data instanceof ArrayBuffer) {
			if (data.byteLength > maxPayloadSize) throw new Error("WebSocket message too large");

			isBinary = true;
			messageData = data;
		} else if (ArrayBuffer.isView(data)) {
			if (data.byteLength > maxPayloadSize) throw new Error("WebSocket message too large");

			isBinary = true;
			const view = data;
			const buffer = view.buffer instanceof SharedArrayBuffer
				? new Uint8Array(view.buffer, view.byteOffset, view.byteLength).slice().buffer
				: view.buffer.slice(view.byteOffset, view.byteOffset + view.byteLength);
			messageData = buffer as ArrayBuffer;
		} else {
			throw new Error("Unsupported data type");
		}

		this.#sendCallback(messageData, isBinary);
	}

	// Called by Tunnel when WebSocket is opened
	_handleOpen(requestId: ArrayBuffer): void {
		if (this.#readyState !== 0) return;
		this.#readyState = 1;
		this.#ws.dispatchEvent({ type: "open", rivetRequestId: requestId, target: this.#ws });
	}

	// Called by Tunnel when message is received
	_handleMessage(
		requestId: ArrayBuffer,
		data: string | Uint8Array,
		messageIndex: number,
		isBinary: boolean,
	): boolean {
		if (this.#readyState !== 1) {
			log(this.#shared)?.warn({
				msg: "WebSocket message ignored - not in OPEN state",
				requestId: this.#requestId,
				actorId: this.#actorId,
				currentReadyState: this.#readyState,
			});
			return true;
		}

		// Validate message index for hibernatable websockets
		if (this.#hibernatable) {
			const previousIndex = this.#messageIndex;

			if (wrappingLteU16(messageIndex, previousIndex)) {
				log(this.#shared)?.info({
					msg: "received duplicate hibernating websocket message",
					requestId,
					actorId: this.#actorId,
					previousIndex,
					receivedIndex: messageIndex,
				});
				return true;
			}

			const expectedIndex = wrappingAddU16(previousIndex, 1);
			if (messageIndex !== expectedIndex) {
				const closeReason = "ws.message_index_skip";
				log(this.#shared)?.warn({
					msg: "hibernatable websocket message index out of sequence, closing connection",
					requestId,
					actorId: this.#actorId,
					previousIndex,
					expectedIndex,
					receivedIndex: messageIndex,
					closeReason,
					gap: wrappingSubU16(wrappingSubU16(messageIndex, previousIndex), 1),
				});
				this.#close(1008, closeReason, true);
				return true;
			}

			this.#messageIndex = messageIndex;
		}

		// Convert data based on binaryType
		let messageData: any = data;
		if (isBinary && data instanceof Uint8Array) {
			if (this.#binaryType === "nodebuffer") {
				messageData = Buffer.from(data);
			} else if (this.#binaryType === "arraybuffer") {
				messageData = data.buffer.slice(data.byteOffset, data.byteOffset + data.byteLength);
			}
		}

		this.#ws.dispatchEvent({
			type: "message",
			data: messageData,
			rivetRequestId: requestId,
			rivetMessageIndex: messageIndex,
			target: this.#ws,
		} as RivetMessageEvent);

		return false;
	}

	// Called by Tunnel when close is received
	_handleClose(_requestId: ArrayBuffer, code?: number, reason?: string): void {
		this.#close(code, reason, true);
	}

	// Close without sending close message to tunnel
	_closeWithoutCallback(code?: number, reason?: string): void {
		this.#close(code, reason, false);
	}

	// Public close method (used by tunnel.ts for stale websocket cleanup)
	close(code?: number, reason?: string): void {
		this.#close(code, reason, true);
	}

	#close(code: number | undefined, reason: string | undefined, sendCallback: boolean): void {
		if (this.#readyState >= 2) return;

		this.#readyState = 2;
		if (sendCallback) this.#closeCallback(code, reason);
		this.#readyState = 3;
		this.#ws.triggerClose(code ?? 1000, reason ?? "");
	}

	#terminate(): void {
		// Immediate close without close frame
		this.#readyState = 3;
		this.#closeCallback(1006, "Abnormal Closure");
		this.#ws.triggerClose(1006, "Abnormal Closure", false);
	}
}
