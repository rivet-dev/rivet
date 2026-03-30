import type * as protocol from "@rivetkit/engine-envoy-protocol";
import type { Logger } from "pino";
import {
	VirtualWebSocket,
	type UniversalWebSocket,
	type RivetMessageEvent,
} from "@rivetkit/virtual-websocket";
import { spawn } from "antiox/task";
import type { EnvoyContext } from "../envoy/index.js";
import { findActiveActor, log } from "../envoy/index.js";
import {
	MAX_PAYLOAD_SIZE,
	idToStr,
	stringifyError,
	wrappingAddU16,
	wrappingLteU16,
	wrappingSubU16,
} from "../../utils.js";
import {
	type RequestEntry,
	requestKey,
	sendTunnelMessage,
	sendTunnelMessageRaw,
} from "./index.js";

export class WebSocketTunnelAdapter {
	#readyState: 0 | 1 | 2 | 3 = 0;
	#ws: VirtualWebSocket;
	#requestKey: string;
	#hibernatable: boolean;
	#serverMessageIndex: number;
	#sendCallback: (data: ArrayBuffer | string, isBinary: boolean) => void;
	#closeCallback: (code?: number, reason?: string) => void;
	#log?: Logger;

	get hibernatable(): boolean {
		return this.#hibernatable;
	}

	get ws(): UniversalWebSocket {
		return this.#ws;
	}

	constructor(
		log: Logger | undefined,
		requestKey: string,
		serverMessageIndex: number,
		hibernatable: boolean,
		isRestoringHibernatable: boolean,
		public readonly request: Request,
		sendCallback: (data: ArrayBuffer | string, isBinary: boolean) => void,
		closeCallback: (code?: number, reason?: string) => void,
	) {
		this.#log = log;
		this.#requestKey = requestKey;
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
			this.#readyState = 1;
		}
	}

	#handleSend(
		data: string | ArrayBufferLike | Blob | ArrayBufferView,
	): void {
		let isBinary = false;
		let messageData: string | ArrayBuffer;

		if (typeof data === "string") {
			if (new TextEncoder().encode(data).byteLength > MAX_PAYLOAD_SIZE) {
				throw new Error("WebSocket message too large");
			}
			messageData = data;
		} else if (data instanceof ArrayBuffer) {
			if (data.byteLength > MAX_PAYLOAD_SIZE)
				throw new Error("WebSocket message too large");
			isBinary = true;
			messageData = data;
		} else if (ArrayBuffer.isView(data)) {
			if (data.byteLength > MAX_PAYLOAD_SIZE)
				throw new Error("WebSocket message too large");
			isBinary = true;
			const view = data;
			const buffer =
				view.buffer instanceof SharedArrayBuffer
					? new Uint8Array(
							view.buffer,
							view.byteOffset,
							view.byteLength,
						).slice().buffer
					: view.buffer.slice(
							view.byteOffset,
							view.byteOffset + view.byteLength,
						);
			messageData = buffer as ArrayBuffer;
		} else {
			throw new Error("Unsupported data type");
		}

		this.#sendCallback(messageData, isBinary);
	}

	handleOpen(requestId: ArrayBuffer): void {
		if (this.#readyState !== 0) return;
		this.#readyState = 1;
		this.#ws.dispatchEvent({
			type: "open",
			rivetRequestId: requestId,
			target: this.#ws,
		});
	}

	handleMessage(
		requestId: ArrayBuffer,
		data: string | Uint8Array,
		serverMessageIndex: number,
		isBinary: boolean,
	): void {
		if (this.#readyState !== 1) {
			this.#log?.warn({
				msg: "WebSocket message ignored, not in OPEN state",
				requestKey: this.#requestKey,
				currentReadyState: this.#readyState,
			});
			return;
		}

		// Validate message index for hibernatable websockets
		if (this.#hibernatable) {
			const previousIndex = this.#serverMessageIndex;

			if (wrappingLteU16(serverMessageIndex, previousIndex)) {
				this.#log?.info({
					msg: "received duplicate hibernating websocket message",
					requestKey: this.#requestKey,
					previousIndex,
					receivedIndex: serverMessageIndex,
				});
				return;
			}

			const expectedIndex = wrappingAddU16(previousIndex, 1);
			if (serverMessageIndex !== expectedIndex) {
				this.#log?.warn({
					msg: "hibernatable websocket message index out of sequence, closing connection",
					requestKey: this.#requestKey,
					previousIndex,
					expectedIndex,
					receivedIndex: serverMessageIndex,
					gap: wrappingSubU16(
						wrappingSubU16(serverMessageIndex, previousIndex),
						1,
					),
				});
				this.#close(1008, "ws.message_index_skip", true);
				return;
			}

			this.#serverMessageIndex = serverMessageIndex;
		}

		// Convert binary data to ArrayBuffer for VirtualWebSocket
		let messageData: any = data;
		if (isBinary && data instanceof Uint8Array) {
			messageData = data.buffer.slice(
				data.byteOffset,
				data.byteOffset + data.byteLength,
			);
		}

		this.#ws.dispatchEvent({
			type: "message",
			data: messageData,
			rivetRequestId: requestId,
			rivetMessageIndex: serverMessageIndex,
			target: this.#ws,
		} as RivetMessageEvent);
	}

	handleClose(
		_requestId: ArrayBuffer,
		code?: number,
		reason?: string,
	): void {
		this.#close(code, reason, true);
	}

	close(code?: number, reason?: string): void {
		this.#close(code, reason, true);
	}

	closeWithoutCallback(code?: number, reason?: string): void {
		this.#close(code, reason, false);
	}

	#close(
		code: number | undefined,
		reason: string | undefined,
		sendCallback: boolean,
	): void {
		if (this.#readyState >= 2) return;

		this.#readyState = 2;
		if (sendCallback) this.#closeCallback(code, reason);
		this.#readyState = 3;
		this.#ws.triggerClose(code ?? 1000, reason ?? "");
	}

	#terminate(): void {
		this.#readyState = 3;
		this.#closeCallback(1006, "Abnormal Closure");
		this.#ws.triggerClose(1006, "Abnormal Closure", false);
	}
}

// MARK: Handlers

export function handleWebSocketOpen(
	ctx: EnvoyContext,
	gatewayId: protocol.GatewayId,
	requestId: protocol.RequestId,
	open: protocol.ToEnvoyWebSocketOpen,
): void {
	const key = requestKey(gatewayId, requestId);
	const requestIdStr = idToStr(requestId);

	const actor = findActiveActor(ctx, open.actorId);
	if (!actor) {
		log(ctx.shared)?.warn({
			msg: "ignoring websocket for unknown actor",
			actorId: open.actorId,
		});
		sendTunnelMessageRaw(ctx, gatewayId, requestId, {
			tag: "ToRivetWebSocketClose",
			val: { code: 1011, reason: "Actor not found", hibernate: false },
		});
		return;
	}

	// Close existing if duplicate open
	const existing = ctx.tunnelRequests.get(key);
	if (existing?.wsAdapter) {
		log(ctx.shared)?.warn({
			msg: "closing existing websocket for duplicate open",
			requestKey: key,
		});
		existing.wsAdapter.closeWithoutCallback(1000, "ws.duplicate_open");
		ctx.tunnelRequests.delete(key);
	}

	// Build request
	const headersObj: Record<string, string> = Object.fromEntries(open.headers);
	const request = buildRequestForWebSocket(open.path, headersObj);

	const canHibernate =
		ctx.shared.config.hibernatableWebSocket.canHibernate(
			open.actorId,
			gatewayId,
			requestId,
			request,
		);

	// Create adapter and entry synchronously to prevent races
	const sendCallback = (
		data: ArrayBuffer | string,
		isBinary: boolean,
	) => {
		const dataBuffer =
			typeof data === "string"
				? (new TextEncoder().encode(data).buffer as ArrayBuffer)
				: data;
		sendTunnelMessage(ctx, gatewayId, requestId, {
			tag: "ToRivetWebSocketMessage",
			val: { data: dataBuffer, binary: isBinary },
		});
	};

	const closeCallback = (code?: number, reason?: string) => {
		sendTunnelMessage(ctx, gatewayId, requestId, {
			tag: "ToRivetWebSocketClose",
			val: {
				code: code ?? null,
				reason: reason ?? null,
				hibernate: false,
			},
		});
		ctx.tunnelRequests.delete(key);
	};

	const adapter = new WebSocketTunnelAdapter(
		log(ctx.shared),
		key,
		0,
		canHibernate,
		false,
		request,
		sendCallback,
		closeCallback,
	);

	const entry: RequestEntry = {
		actorId: open.actorId,
		generation: actor.generation,
		gatewayId,
		requestId,
		clientMessageIndex: 0,
		wsAdapter: adapter,
	};
	ctx.tunnelRequests.set(key, entry);

	const capturedEntry = entry;
	const actorStartPromise = actor.entry.actorStartPromise;
	const actorId = open.actorId;

	spawn(async () => {
		try {
			await actorStartPromise;

			if (ctx.shuttingDown) return;
			if (ctx.tunnelRequests.get(key) !== capturedEntry) return;

			await ctx.shared.config.websocket(
				ctx.shared.handle,
				actorId,
				adapter.ws,
				gatewayId,
				requestId,
				request,
				open.path,
				headersObj,
				canHibernate,
				false,
			);

			if (ctx.tunnelRequests.get(key) !== capturedEntry) return;

			// Notify the gateway that the websocket is open
			sendTunnelMessage(ctx, gatewayId, requestId, {
				tag: "ToRivetWebSocketOpen",
				val: { canHibernate },
			});

			// Dispatch open event to user code
			adapter.handleOpen(requestId);
		} catch (error) {
			if (ctx.tunnelRequests.get(key) !== capturedEntry) return;

			log(ctx.shared)?.error({
				msg: "error handling websocket open",
				actorId,
				requestId: requestIdStr,
				error: stringifyError(error),
			});

			// Send close with error
			sendTunnelMessage(ctx, gatewayId, requestId, {
				tag: "ToRivetWebSocketClose",
				val: { code: 1011, reason: "Server Error", hibernate: false },
			});
			ctx.tunnelRequests.delete(key);
		}
	});
}

export function handleWebSocketMessage(
	ctx: EnvoyContext,
	gatewayId: protocol.GatewayId,
	requestId: protocol.RequestId,
	serverMessageIndex: number,
	msg: protocol.ToEnvoyWebSocketMessage,
): void {
	const key = requestKey(gatewayId, requestId);
	const entry = ctx.tunnelRequests.get(key);

	if (!entry?.wsAdapter) {
		log(ctx.shared)?.warn({
			msg: "missing websocket for incoming message",
			requestKey: key,
		});
		return;
	}

	const data = msg.binary
		? new Uint8Array(msg.data)
		: new TextDecoder().decode(new Uint8Array(msg.data));

	entry.wsAdapter.handleMessage(requestId, data, serverMessageIndex, msg.binary);
}

export function handleWebSocketClose(
	ctx: EnvoyContext,
	gatewayId: protocol.GatewayId,
	requestId: protocol.RequestId,
	close: protocol.ToEnvoyWebSocketClose,
): void {
	const key = requestKey(gatewayId, requestId);
	const entry = ctx.tunnelRequests.get(key);

	if (!entry?.wsAdapter) {
		log(ctx.shared)?.warn({
			msg: "missing websocket for incoming close",
			requestKey: key,
		});
		return;
	}

	entry.wsAdapter.handleClose(
		requestId,
		close.code ?? undefined,
		close.reason ?? undefined,
	);
}

// MARK: Util

function buildRequestForWebSocket(
	path: string,
	headers: Record<string, string>,
): Request {
	const fullHeaders = {
		...headers,
		Upgrade: "websocket",
		Connection: "Upgrade",
	};

	if (!path.startsWith("/")) {
		throw new Error("Path must start with leading slash");
	}

	return new Request(`http://actor${path}`, {
		method: "GET",
		headers: fullHeaders,
	});
}
