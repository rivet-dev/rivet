import type { Logger } from "pino";
import { VirtualWebSocket, type UniversalWebSocket, type RivetMessageEvent } from "@rivetkit/virtual-websocket";
import type { Tunnel } from "./tunnel";
import { MAX_BODY_SIZE, wrappingAddU16, wrappingLteU16, wrappingSubU16 } from "./utils";

export const HIBERNATABLE_SYMBOL = Symbol("hibernatable");

export class WebSocketTunnelAdapter {
	#readyState: 0 | 1 | 2 | 3 = 0;
	#binaryType: "nodebuffer" | "arraybuffer" | "blob" = "nodebuffer";
	#ws: VirtualWebSocket;
	#tunnel: Tunnel;
	#actorId: string;
	#requestId: string;
	#hibernatable: boolean;
	#serverMessageIndex: number;
	#sendCallback: (data: ArrayBuffer | string, isBinary: boolean) => void;
	#closeCallback: (code?: number, reason?: string) => void;

	get [HIBERNATABLE_SYMBOL](): boolean {
		return this.#hibernatable;
	}

	get #log(): Logger | undefined {
		return this.#tunnel.log;
	}

	constructor(
		tunnel: Tunnel,
		actorId: string,
		requestId: string,
		serverMessageIndex: number,
		hibernatable: boolean,
		isRestoringHibernatable: boolean,
		public readonly request: Request,
		sendCallback: (data: ArrayBuffer | string, isBinary: boolean) => void,
		closeCallback: (code?: number, reason?: string) => void,
	) {
		this.#tunnel = tunnel;
		this.#actorId = actorId;
		this.#requestId = requestId;
		this.#hibernatable = hibernatable;
		this.#serverMessageIndex = serverMessageIndex;
		this.#sendCallback = sendCallback;
		this.#closeCallback = closeCallback;

		this.#ws = new VirtualWebSocket({
			getReadyState: () => this.#readyState,
			onSend: (data) => this.#handleSend(data),
			onClose: (code, reason) => this.#close(code, reason, true),
			onTerminate: () => this.#terminate(),
		});

		if (isRestoringHibernatable) {
			this.#log?.debug({
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

		if (typeof data === "string") {
			const encoder = new TextEncoder();
			if (encoder.encode(data).byteLength > MAX_BODY_SIZE) {
				throw new Error("WebSocket message too large");
			}

			messageData = data;
		} else if (data instanceof ArrayBuffer) {
			if (data.byteLength > MAX_BODY_SIZE) throw new Error("WebSocket message too large");

			isBinary = true;
			messageData = data;
		} else if (ArrayBuffer.isView(data)) {
			if (data.byteLength > MAX_BODY_SIZE) throw new Error("WebSocket message too large");

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
		serverMessageIndex: number,
		isBinary: boolean,
	): boolean {
		if (this.#readyState !== 1) {
			this.#log?.warn({
				msg: "WebSocket message ignored - not in OPEN state",
				requestId: this.#requestId,
				actorId: this.#actorId,
				currentReadyState: this.#readyState,
			});
			return true;
		}

		// Validate message index for hibernatable websockets
		if (this.#hibernatable) {
			const previousIndex = this.#serverMessageIndex;

			if (wrappingLteU16(serverMessageIndex, previousIndex)) {
				this.#log?.info({
					msg: "received duplicate hibernating websocket message",
					requestId,
					actorId: this.#actorId,
					previousIndex,
					receivedIndex: serverMessageIndex,
				});
				return true;
			}

			const expectedIndex = wrappingAddU16(previousIndex, 1);
			if (serverMessageIndex !== expectedIndex) {
				const closeReason = "ws.message_index_skip";
				this.#log?.warn({
					msg: "hibernatable websocket message index out of sequence, closing connection",
					requestId,
					actorId: this.#actorId,
					previousIndex,
					expectedIndex,
					receivedIndex: serverMessageIndex,
					closeReason,
					gap: wrappingSubU16(wrappingSubU16(serverMessageIndex, previousIndex), 1),
				});
				this.#close(1008, closeReason, true);
				return true;
			}

			this.#serverMessageIndex = serverMessageIndex;
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
			rivetMessageIndex: serverMessageIndex,
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
