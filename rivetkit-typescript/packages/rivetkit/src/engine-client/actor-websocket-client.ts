import {
	type ClientConfig,
	DEFAULT_MAX_QUERY_INPUT_SIZE,
} from "@/client/config";
import {
	WS_PROTOCOL_CONN_PARAMS,
	WS_PROTOCOL_ENCODING,
	WS_PROTOCOL_STANDARD as WS_PROTOCOL_RIVETKIT,
	WS_PROTOCOL_TARGET,
	WS_PROTOCOL_ACTOR,
	WS_PROTOCOL_BYPASS_CONNECTABLE,
	WS_PROTOCOL_TEST_ACK_HOOK,
	WS_PROTOCOL_TOKEN,
} from "@/common/actor-router-consts";
import { importWebSocket } from "@/common/websocket";
import { setRemoteHibernatableWebSocketAckTestHooks } from "@/common/websocket-test-hooks";
import type { ActorGatewayQuery, CrashPolicy } from "@/client/query";
import type { Encoding, UniversalWebSocket } from "@/mod";
import { encodeCborCompat, uint8ArrayToBase64 } from "@/serde";
import { combineUrlPath } from "@/utils";
import type { GatewayRequestOptions } from "./driver";
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

export function buildActorQueryGatewayUrl(
	endpoint: string,
	namespace: string,
	query: ActorGatewayQuery,
	token: string | undefined,
	path = "",
	maxInputSize = DEFAULT_MAX_QUERY_INPUT_SIZE,
	crashPolicy: CrashPolicy | undefined = undefined,
	runnerName?: string,
	options: GatewayRequestOptions = {},
): string {
	if (namespace.length === 0) {
		throw new Error("actor query namespace must not be empty");
	}

	let name: string;
	const params = new URLSearchParams();
	params.append("rvt-namespace", namespace);

	if ("getForKey" in query) {
		name = query.getForKey.name;
		params.append("rvt-method", "get");
		pushKeyQueryParams(params, query.getForKey.key);
		if (crashPolicy !== undefined) {
			throw new Error(
				"Actor query method=get does not support crashPolicy.",
			);
		}
		if (runnerName !== undefined) {
			throw new Error(
				"Actor query method=get does not support runnerName.",
			);
		}
	} else if ("getOrCreateForKey" in query) {
		name = query.getOrCreateForKey.name;
		params.append("rvt-method", "getOrCreate");
		if (runnerName === undefined) {
			throw new Error(
				"Actor query method=getOrCreate requires runnerName.",
			);
		}
		params.append("rvt-runner", runnerName);
		pushKeyQueryParams(params, query.getOrCreateForKey.key);
		pushInputQueryParam(
			params,
			query.getOrCreateForKey.input,
			maxInputSize,
		);
		if (query.getOrCreateForKey.region !== undefined) {
			params.append("rvt-region", query.getOrCreateForKey.region);
		}
		params.append("rvt-crash-policy", crashPolicy ?? "sleep");
	} else {
		throw new Error(
			"Actor query gateway URLs only support get and getOrCreate.",
		);
	}

	if (name.length === 0) {
		throw new Error("actor query name must not be empty");
	}

	if (token !== undefined) {
		params.append("rvt-token", token);
	}
	if (options.bypassConnectable) {
		params.append("rvt-bypass_connectable", "true");
	}

	const queryString = params.toString();
	let separator: string;
	if (path.endsWith("?") || path.endsWith("&")) {
		separator = "";
	} else if (path.includes("?")) {
		separator = "&";
	} else {
		separator = "?";
	}
	const gatewayPath = `/gateway/${encodeURIComponent(name)}${path}${separator}${queryString}`;

	return combineUrlPath(endpoint, gatewayPath);
}

function pushKeyQueryParams(params: URLSearchParams, key: string[]): void {
	if (key.length > 0) {
		params.append("rvt-key", key.join(","));
	}
}

function pushInputQueryParam(
	params: URLSearchParams,
	input: unknown,
	maxInputSize: number,
): void {
	if (input === undefined) {
		return;
	}

	const encodedInput = encodeCborCompat(input);
	if (encodedInput.byteLength > maxInputSize) {
		throw new Error(
			`Actor query input exceeds maxInputSize (${encodedInput.byteLength} > ${maxInputSize} bytes). Increase client maxInputSize to allow larger query payloads.`,
		);
	}

	params.append("rvt-input", uint8ArrayToBase64Url(encodedInput));
}

function uint8ArrayToBase64Url(value: Uint8Array): string {
	return uint8ArrayToBase64(value)
		.replace(/\+/g, "-")
		.replace(/\//g, "_")
		.replace(/=+$/g, "");
}

export async function openWebSocketToGateway(
	runConfig: ClientConfig,
	gatewayUrl: string,
	encoding: Encoding,
	params: unknown,
	options: GatewayRequestOptions & { directActorId?: string } = {},
): Promise<UniversalWebSocket> {
	const WebSocket = await importWebSocket();

	const ackHookToken =
		typeof process !== "undefined" && process.env.VITEST
			? crypto.randomUUID()
			: undefined;

	logger().debug({
		msg: "opening websocket to actor via guard",
		gatewayUrl,
	});

	// Create WebSocket connection
	const ws = new WebSocket(
		gatewayUrl,
		buildWebSocketProtocols(
			runConfig,
			encoding,
			params,
			ackHookToken,
			options.directActorId
				? {
						target: "actor",
						actorId: options.directActorId,
					}
				: undefined,
			options,
		),
	);

	// The WebSocket is returned before the connection is open. This follows
	// standard WebSocket behavior where the caller listens for the "open"
	// event before sending messages.
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

	return bufferedWs;
}

export function buildWebSocketProtocols(
	_runConfig: ClientConfig,
	encoding: Encoding,
	params?: unknown,
	ackHookToken?: string,
	target?: {
		target: "actor";
		actorId: string;
	},
	options: GatewayRequestOptions = {},
): string[] {
	const protocols: string[] = [];
	protocols.push(WS_PROTOCOL_RIVETKIT);
	protocols.push(`${WS_PROTOCOL_ENCODING}${encoding}`);
	if (target) {
		protocols.push(`${WS_PROTOCOL_TARGET}${target.target}`);
		protocols.push(`${WS_PROTOCOL_ACTOR}${target.actorId}`);
	}
	if (options.bypassConnectable) {
		protocols.push(WS_PROTOCOL_BYPASS_CONNECTABLE);
	}
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
