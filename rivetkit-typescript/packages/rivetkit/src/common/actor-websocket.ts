import type { WSContext } from "hono/ws";
import {
	WS_PROTOCOL_CONN_PARAMS,
	WS_PROTOCOL_ENCODING,
	WS_PROTOCOL_TEST_ACK_HOOK,
} from "@/common/actor-router-consts";
import { type Encoding, EncodingSchema } from "./encoding";

export interface UpgradeWebSocketArgs {
	conn?: unknown;
	actor?: unknown;
	onRestore?: (ws: WSContext) => void;
	onOpen: (event: any, ws: WSContext) => void;
	onMessage: (event: any, ws: WSContext) => void;
	onClose: (event: any, ws: WSContext) => void;
	onError: (error: any, ws: WSContext) => void;
}

export interface WebSocketCustomProtocols {
	encoding: Encoding;
	connParams: unknown;
	ackHookToken?: string;
}

export function parseWebSocketProtocols(
	protocols: string | null | undefined,
): WebSocketCustomProtocols {
	let encodingRaw: string | undefined;
	let connParamsRaw: string | undefined;
	let ackHookTokenRaw: string | undefined;

	if (protocols) {
		for (const protocol of protocols.split(",").map((value) => value.trim())) {
			if (protocol.startsWith(WS_PROTOCOL_ENCODING)) {
				encodingRaw = protocol.substring(WS_PROTOCOL_ENCODING.length);
			} else if (protocol.startsWith(WS_PROTOCOL_CONN_PARAMS)) {
				connParamsRaw = decodeURIComponent(
					protocol.substring(WS_PROTOCOL_CONN_PARAMS.length),
				);
			} else if (protocol.startsWith(WS_PROTOCOL_TEST_ACK_HOOK)) {
				ackHookTokenRaw = decodeURIComponent(
					protocol.substring(WS_PROTOCOL_TEST_ACK_HOOK.length),
				);
			}
		}
	}

	return {
		encoding: EncodingSchema.parse(encodingRaw ?? "json"),
		connParams: connParamsRaw ? JSON.parse(connParamsRaw) : undefined,
		ackHookToken: ackHookTokenRaw,
	};
}

export function truncateRawWebSocketPathPrefix(path: string): string {
	const url = new URL(path, "http://actor");
	const pathname = url.pathname.replace(/^\/websocket\/?/, "") || "/";
	return (pathname.startsWith("/") ? pathname : `/${pathname}`) + url.search;
}
