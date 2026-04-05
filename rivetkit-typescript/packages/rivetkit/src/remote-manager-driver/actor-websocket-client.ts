import * as cbor from "cbor-x";
import {
	type ClientConfig,
	DEFAULT_MAX_QUERY_INPUT_SIZE,
} from "@/client/config";
import {
	WS_PROTOCOL_CONN_PARAMS,
	WS_PROTOCOL_ENCODING,
	WS_PROTOCOL_STANDARD as WS_PROTOCOL_RIVETKIT,
} from "@/common/actor-router-consts";
import { importWebSocket } from "@/common/websocket";
import type { ActorGatewayQuery, CrashPolicy } from "@/manager/protocol/query";
import type { Encoding, UniversalWebSocket } from "@/mod";
import { uint8ArrayToBase64 } from "@/serde";
import { combineUrlPath } from "@/utils";
import { logger } from "./log";

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
		pushInputQueryParam(params, query.getOrCreateForKey.input, maxInputSize);
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

function pushKeyQueryParams(
	params: URLSearchParams,
	key: string[],
): void {
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

	const encodedInput = cbor.encode(input);
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
): Promise<UniversalWebSocket> {
	const WebSocket = await importWebSocket();

	logger().debug({
		msg: "opening websocket to actor via guard",
		gatewayUrl,
	});

	// Create WebSocket connection
	const ws = new WebSocket(
		gatewayUrl,
		buildWebSocketProtocols(runConfig, encoding, params),
	);

	// Set binary type to arraybuffer for proper encoding support
	ws.binaryType = "arraybuffer";

	await waitForWebSocketOpen(ws as UniversalWebSocket);

	logger().debug({ msg: "websocket connection ready", gatewayUrl });

	return ws as UniversalWebSocket;
}

async function waitForWebSocketOpen(
	ws: UniversalWebSocket,
): Promise<void> {
	await new Promise<void>((resolve, reject) => {
		let settled = false;

		const cleanup = () => {
			removeWsListener(ws, "open", onOpen);
			removeWsListener(ws, "error", onError);
			removeWsListener(ws, "close", onClose);
		};

		const settleResolve = () => {
			if (settled) return;
			settled = true;
			cleanup();
			resolve();
		};

		const settleReject = (error: Error) => {
			if (settled) return;
			settled = true;
			cleanup();
			reject(error);
		};

		const onOpen = () => {
			settleResolve();
		};
		const onError = (event: unknown) => {
			settleReject(webSocketOpenError(event));
		};
		const onClose = (event: unknown) => {
			settleReject(webSocketCloseError(event));
		};

		addWsListener(ws, "open", onOpen);
		addWsListener(ws, "error", onError);
		addWsListener(ws, "close", onClose);
	});
}

function addWsListener(
	ws: UniversalWebSocket,
	event: "open" | "error" | "close",
	handler: (event?: unknown) => void,
): void {
	if ("addEventListener" in ws && typeof ws.addEventListener === "function") {
		ws.addEventListener(event, handler as EventListener, { once: true });
		return;
	}

	if ("once" in ws && typeof (ws as { once?: unknown }).once === "function") {
		(ws as { once(event: string, handler: (event?: unknown) => void): void })
			.once(event, handler);
		return;
	}

	if ("on" in ws && typeof (ws as { on?: unknown }).on === "function") {
		(ws as { on(event: string, handler: (event?: unknown) => void): void })
			.on(event, handler);
	}
}

function removeWsListener(
	ws: UniversalWebSocket,
	event: "open" | "error" | "close",
	handler: (event?: unknown) => void,
): void {
	if (
		"removeEventListener" in ws &&
		typeof ws.removeEventListener === "function"
	) {
		ws.removeEventListener(event, handler as EventListener);
		return;
	}

	if ("off" in ws && typeof (ws as { off?: unknown }).off === "function") {
		(ws as { off(event: string, handler: (event?: unknown) => void): void })
			.off(event, handler);
		return;
	}

	if (
		"removeListener" in ws &&
		typeof (ws as { removeListener?: unknown }).removeListener === "function"
	) {
		(
			ws as {
				removeListener(
					event: string,
					handler: (event?: unknown) => void,
				): void;
			}
		).removeListener(event, handler);
	}
}

function webSocketOpenError(event: unknown): Error {
	if (event instanceof Error) {
		return event;
	}

	if (
		typeof event === "object" &&
		event !== null &&
		"error" in event &&
		event.error instanceof Error
	) {
		return event.error;
	}

	return new Error("WebSocket failed before opening");
}

function webSocketCloseError(event: unknown): Error {
	if (
		typeof event === "object" &&
		event !== null &&
		"reason" in event &&
		typeof event.reason === "string" &&
		event.reason.length > 0
	) {
		return new Error(event.reason);
	}

	if (
		typeof event === "object" &&
		event !== null &&
		"code" in event &&
		typeof event.code === "number"
	) {
		return new Error(`WebSocket closed before opening (code ${event.code})`);
	}

	return new Error("WebSocket closed before opening");
}

export function buildWebSocketProtocols(
	_runConfig: ClientConfig,
	encoding: Encoding,
	params?: unknown,
): string[] {
	const protocols: string[] = [];
	protocols.push(WS_PROTOCOL_RIVETKIT);
	protocols.push(`${WS_PROTOCOL_ENCODING}${encoding}`);
	if (params) {
		protocols.push(
			`${WS_PROTOCOL_CONN_PARAMS}${encodeURIComponent(JSON.stringify(params))}`,
		);
	}
	return protocols;
}
