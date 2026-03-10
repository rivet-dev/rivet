import type { ClientConfig } from "@/client/config";
import {
	HEADER_CONN_PARAMS,
	HEADER_ENCODING,
	WS_PROTOCOL_CONN_PARAMS,
	WS_PROTOCOL_ENCODING,
	WS_PROTOCOL_STANDARD as WS_PROTOCOL_RIVETKIT,
	WS_PROTOCOL_TEST_ACK_HOOK,
	WS_PROTOCOL_TOKEN,
} from "@/common/actor-router-consts";
import { importWebSocket } from "@/common/websocket";
import { setRemoteHibernatableWebSocketAckTestHooks } from "@/common/websocket-test-hooks";
import type { Encoding, UniversalWebSocket } from "@/mod";
import { combineUrlPath } from "@/utils";
import { getEndpoint } from "./api-utils";
import { logger } from "./log";

class BufferedRemoteWebSocket implements UniversalWebSocket {
	readonly CONNECTING = 0 as const;
	readonly OPEN = 1 as const;
	readonly CLOSING = 2 as const;
	readonly CLOSED = 3 as const;

	#inner: UniversalWebSocket;
	#listeners = new Map<string, Set<(event: any) => void>>();
	#queuedEvents: Array<{ type: string; event: any }> = [];
	#onopen: ((event: any) => void) | null = null;
	#onclose: ((event: any) => void) | null = null;
	#onerror: ((event: any) => void) | null = null;
	#onmessage: ((event: any) => void) | null = null;

	constructor(inner: UniversalWebSocket) {
		this.#inner = inner;
		for (const type of ["open", "message", "close", "error"]) {
			this.#inner.addEventListener(type, (event) => {
				this.#handleEvent(type, event);
			});
		}
	}

	get readyState() {
		return this.#inner.readyState;
	}

	get binaryType() {
		return this.#inner.binaryType;
	}

	set binaryType(value) {
		this.#inner.binaryType = value;
	}

	get bufferedAmount() {
		return this.#inner.bufferedAmount;
	}

	get extensions() {
		return this.#inner.extensions;
	}

	get protocol() {
		return this.#inner.protocol;
	}

	get url() {
		return this.#inner.url;
	}

	get onopen() {
		return this.#onopen;
	}

	set onopen(handler) {
		this.#onopen = handler;
		this.#flushQueuedEvents();
	}

	get onclose() {
		return this.#onclose;
	}

	set onclose(handler) {
		this.#onclose = handler;
		this.#flushQueuedEvents();
	}

	get onerror() {
		return this.#onerror;
	}

	set onerror(handler) {
		this.#onerror = handler;
		this.#flushQueuedEvents();
	}

	get onmessage() {
		return this.#onmessage;
	}

	set onmessage(handler) {
		this.#onmessage = handler;
		this.#flushQueuedEvents();
	}

	send(data: string | ArrayBufferLike | Blob | ArrayBufferView): void {
		this.#inner.send(data);
	}

	close(code?: number, reason?: string): void {
		this.#inner.close(code, reason);
	}

	addEventListener(type: string, listener: (event: any) => void): void {
		const listeners = this.#listeners.get(type) ?? new Set();
		listeners.add(listener);
		this.#listeners.set(type, listeners);
		this.#flushQueuedEvents();
	}

	removeEventListener(type: string, listener: (event: any) => void): void {
		this.#listeners.get(type)?.delete(listener);
	}

	dispatchEvent(event: any): boolean {
		return this.#dispatchEvent(event.type, event);
	}

	#handleEvent(type: string, event: any): void {
		if (this.#hasConsumer(type)) {
			this.#dispatchEvent(type, event);
			return;
		}
		this.#queuedEvents.push({ type, event });
	}

	#flushQueuedEvents(): void {
		if (this.#queuedEvents.length === 0) {
			return;
		}

		const pending = this.#queuedEvents;
		this.#queuedEvents = [];
		for (const pendingEvent of pending) {
			if (this.#hasConsumer(pendingEvent.type)) {
				this.#dispatchEvent(pendingEvent.type, pendingEvent.event);
				continue;
			}
			this.#queuedEvents.push(pendingEvent);
		}
	}

	#hasConsumer(type: string): boolean {
		const handler =
			type === "open"
				? this.#onopen
				: type === "close"
					? this.#onclose
					: type === "error"
						? this.#onerror
						: type === "message"
							? this.#onmessage
							: null;
		return Boolean(handler) || (this.#listeners.get(type)?.size ?? 0) > 0;
	}

	#dispatchEvent(type: string, event: any): boolean {
		const listeners = this.#listeners.get(type);
		if (listeners) {
			for (const listener of listeners) {
				listener(event);
			}
		}

		const handler =
			type === "open"
				? this.#onopen
				: type === "close"
					? this.#onclose
					: type === "error"
						? this.#onerror
						: type === "message"
							? this.#onmessage
							: null;
		handler?.(event);
		return true;
	}
}

export function buildActorGatewayUrl(
	endpoint: string,
	actorId: string,
	token: string | undefined,
	path = "",
): string {
	const tokenSegment =
		token !== undefined ? `@${encodeURIComponent(token)}` : "";
	const gatewayPath = `/gateway/${encodeURIComponent(actorId)}${tokenSegment}${path}`;
	return combineUrlPath(endpoint, gatewayPath);
}

export async function openWebSocketToActor(
	runConfig: ClientConfig,
	path: string,
	actorId: string,
	encoding: Encoding,
	params: unknown,
): Promise<UniversalWebSocket> {
	const WebSocket = await importWebSocket();

	// WebSocket connections go through guard
	const endpoint = getEndpoint(runConfig);
	const guardUrl = buildActorGatewayUrl(
		endpoint,
		actorId,
		runConfig.token,
		path,
	);
	const ackHookToken =
		typeof process !== "undefined" && process.env.VITEST
			? crypto.randomUUID()
			: undefined;

	logger().debug({
		msg: "opening websocket to actor via guard",
		actorId,
		path,
		guardUrl,
	});

	// Create WebSocket connection
	const ws = new WebSocket(
		guardUrl,
		buildWebSocketProtocols(runConfig, encoding, params, ackHookToken),
	);

	// Set binary type to arraybuffer for proper encoding support
	ws.binaryType = "arraybuffer";
	const bufferedWs = new BufferedRemoteWebSocket(
		ws as unknown as UniversalWebSocket,
	);
	if (ackHookToken) {
		setRemoteHibernatableWebSocketAckTestHooks(
			bufferedWs,
			ackHookToken,
			true,
		);
	}

	logger().debug({ msg: "websocket connection opened", actorId });

	return bufferedWs;
}

export function buildWebSocketProtocols(
	runConfig: ClientConfig,
	encoding: Encoding,
	params?: unknown,
	ackHookToken?: string,
): string[] {
	const protocols: string[] = [];
	protocols.push(WS_PROTOCOL_RIVETKIT);
	protocols.push(`${WS_PROTOCOL_ENCODING}${encoding}`);
	if (params) {
		protocols.push(
			`${WS_PROTOCOL_CONN_PARAMS}${encodeURIComponent(JSON.stringify(params))}`,
		);
	}
	if (ackHookToken) {
		protocols.push(
			`${WS_PROTOCOL_TEST_ACK_HOOK}${encodeURIComponent(ackHookToken)}`,
		);
	}
	return protocols;
}
