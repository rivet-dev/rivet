import * as protocol from "@rivetkit/engine-envoy-protocol";
import type { EnvoyContext } from "../envoy/index.js";
import { log, wsSend } from "../envoy/index.js";
import { idToStr } from "../../utils.js";
import {
	handleRequestStart,
	handleRequestChunk,
	handleRequestAbort,
} from "./http.js";
import {
	handleWebSocketOpen,
	handleWebSocketMessage,
	handleWebSocketClose,
} from "./websocket.js";
import type { WebSocketTunnelAdapter } from "./websocket.js";
import { unreachable } from "antiox/panic";

export interface RequestEntry {
	actorId: string;
	generation: number;
	gatewayId: protocol.GatewayId;
	requestId: protocol.RequestId;
	clientMessageIndex: number;
	streamController?: ReadableStreamDefaultController<Uint8Array>;
	wsAdapter?: WebSocketTunnelAdapter;
}

export interface BufferedTunnelMessage {
	gatewayId: protocol.GatewayId;
	requestId: protocol.RequestId;
	messageKind: protocol.ToRivetTunnelMessageKind;
}

export function requestKey(
	gatewayId: protocol.GatewayId,
	requestId: protocol.RequestId,
): string {
	return `${idToStr(gatewayId)}:${idToStr(requestId)}`;
}

export function handleTunnelMessage(
	ctx: EnvoyContext,
	message: protocol.ToEnvoyTunnelMessage,
) {
	const { gatewayId, requestId, messageIndex } = message.messageId;
	const { messageKind } = message;

	switch (messageKind.tag) {
		case "ToEnvoyRequestStart":
			handleRequestStart(ctx, gatewayId, requestId, messageKind.val);
			break;
		case "ToEnvoyRequestChunk":
			handleRequestChunk(ctx, gatewayId, requestId, messageKind.val);
			break;
		case "ToEnvoyRequestAbort":
			handleRequestAbort(ctx, gatewayId, requestId);
			break;
		case "ToEnvoyWebSocketOpen":
			handleWebSocketOpen(ctx, gatewayId, requestId, messageKind.val);
			break;
		case "ToEnvoyWebSocketMessage":
			handleWebSocketMessage(
				ctx,
				gatewayId,
				requestId,
				messageIndex,
				messageKind.val,
			);
			break;
		case "ToEnvoyWebSocketClose":
			handleWebSocketClose(ctx, gatewayId, requestId, messageKind.val);
			break;
		default:
			unreachable(messageKind);
	}
}

/**
 * Sends a tunnel message with a tracked clientMessageIndex from the
 * RequestEntry. If the websocket is disconnected, the message is buffered for
 * resend on reconnect. If the entry no longer exists, the message is dropped.
 */
export function sendTunnelMessage(
	ctx: EnvoyContext,
	gatewayId: protocol.GatewayId,
	requestId: protocol.RequestId,
	messageKind: protocol.ToRivetTunnelMessageKind,
): void {
	if (!ctx.shared.wsTx) {
		ctx.bufferedTunnelMessages.push({ gatewayId, requestId, messageKind });
		return;
	}

	const key = requestKey(gatewayId, requestId);
	const entry = ctx.tunnelRequests.get(key);
	if (!entry) {
		log(ctx.shared)?.warn({
			msg: "cannot send tunnel message, request entry not found",
			requestKey: key,
		});
		return;
	}

	const messageIndex = entry.clientMessageIndex;
	entry.clientMessageIndex++;

	wsSend(ctx, {
		tag: "ToRivetTunnelMessage",
		val: {
			messageId: { gatewayId, requestId, messageIndex },
			messageKind,
		},
	});
}

/**
 * Sends a tunnel message with messageIndex 0 without requiring a
 * RequestEntry. Used for one-shot error responses (503, 1011) when no request
 * lifecycle exists.
 */
export function sendTunnelMessageRaw(
	ctx: EnvoyContext,
	gatewayId: protocol.GatewayId,
	requestId: protocol.RequestId,
	messageKind: protocol.ToRivetTunnelMessageKind,
): void {
	// Drop raw messages when disconnected. The gateway will retry.
	if (!ctx.shared.wsTx) return;

	wsSend(ctx, {
		tag: "ToRivetTunnelMessage",
		val: {
			messageId: { gatewayId, requestId, messageIndex: 0 },
			messageKind,
		},
	});
}

export function resendBufferedTunnelMessages(ctx: EnvoyContext): void {
	if (ctx.bufferedTunnelMessages.length === 0) return;

	log(ctx.shared)?.info({
		msg: "resending buffered tunnel messages",
		count: ctx.bufferedTunnelMessages.length,
	});

	const messages = ctx.bufferedTunnelMessages;
	ctx.bufferedTunnelMessages = [];

	for (const { gatewayId, requestId, messageKind } of messages) {
		sendTunnelMessage(ctx, gatewayId, requestId, messageKind);
	}
}

export function shutdownTunnel(ctx: EnvoyContext): void {
	ctx.shuttingDown = true;

	for (const entry of ctx.tunnelRequests.values()) {
		if (entry.streamController) {
			try {
				entry.streamController.error(new Error("envoy shutting down"));
			} catch {
				// Controller may already be closed
			}
		}
		if (entry.wsAdapter && !entry.wsAdapter.hibernatable) {
			entry.wsAdapter.closeWithoutCallback(1000, "ws.tunnel_shutdown");
		}
	}

	ctx.tunnelRequests.clear();
	ctx.bufferedTunnelMessages = [];
}

export function closeTunnelRequestsForActor(
	ctx: EnvoyContext,
	actorId: string,
): void {
	const keysToRemove: string[] = [];

	for (const [key, entry] of ctx.tunnelRequests) {
		if (entry.actorId !== actorId) continue;
		keysToRemove.push(key);

		if (entry.streamController) {
			try {
				entry.streamController.error(new Error("actor stopped"));
			} catch {
				// Controller may already be closed
			}
		}
		if (entry.wsAdapter && !entry.wsAdapter.hibernatable) {
			entry.wsAdapter.closeWithoutCallback(1000, "actor.stopped");
		}
	}

	for (const key of keysToRemove) {
		ctx.tunnelRequests.delete(key);
	}
}
