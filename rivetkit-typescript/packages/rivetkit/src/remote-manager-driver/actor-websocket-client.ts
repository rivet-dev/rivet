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

	logger().debug({ msg: "websocket connection opened", gatewayUrl });

	return ws as UniversalWebSocket;
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
