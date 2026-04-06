import * as protocol from "@rivetkit/engine-envoy-protocol";
import { EnvoyContext, getActor, log } from "./index.js";
import { SharedContext } from "@/context.js";
import { unreachable } from "antiox";
import { wsSend } from "../connection.js";
import { idToStr } from "@/utils.js";
import { stringifyToRivetTunnelMessageKind } from "@/stringify.js";

export interface HibernatingWebSocketMetadata {
	gatewayId: protocol.GatewayId;
	requestId: protocol.RequestId;
	envoyMessageIndex: number;
	rivetMessageIndex: number;

	path: string;
	headers: Record<string, string>;
}

export function handleTunnelMessage(ctx: EnvoyContext, msg: protocol.ToEnvoyTunnelMessage) {
	const {
		messageId,
		messageKind: { tag, val },
	} = msg;

	if (tag === "ToEnvoyRequestStart") {
		handleRequestStart(ctx, messageId, val);
	} else if (tag === "ToEnvoyRequestChunk") {
		handleRequestChunk(ctx, messageId, val);
	} else if (tag === "ToEnvoyRequestAbort") {
		handleRequestAbort(ctx, messageId);
	} else if (tag === "ToEnvoyWebSocketOpen") {
		handleWebSocketOpen(ctx, messageId, val);
	} else if (tag === "ToEnvoyWebSocketMessage") {
		handleWebSocketMessage(ctx, messageId, val);
	} else if (tag === "ToEnvoyWebSocketClose") {
		handleWebSocketClose(ctx, messageId, val);
	} else {
		unreachable(tag);
	}
}

function handleRequestStart(ctx: EnvoyContext, messageId: protocol.MessageId, req: protocol.ToEnvoyRequestStart) {
	const actor = getActor(ctx, req.actorId);

	if (!actor) {
		log(ctx.shared)?.warn({
			msg: "received request for unknown actor",
			actorId: req.actorId,
		});

		sendErrorResponse(ctx, messageId.gatewayId, messageId.requestId);

		return;
	}

	ctx.requestToActor.set([messageId.gatewayId, messageId.requestId], req.actorId);

	actor.handle.send({
		type: "req-start",
		messageId,
		req,
	});
}

function handleRequestChunk(ctx: EnvoyContext, messageId: protocol.MessageId, chunk: protocol.ToEnvoyRequestChunk) {
	const actorId = ctx.requestToActor.get([messageId.gatewayId, messageId.requestId]);
	if (actorId) {
		let actor = getActor(ctx, actorId);
		if (actor) {
			actor.handle.send({ type: "req-chunk", messageId, chunk });
		}
	}

	if (chunk.finish) {
		ctx.requestToActor.delete([messageId.gatewayId, messageId.requestId]);
	}
}

function handleRequestAbort(ctx: EnvoyContext, messageId: protocol.MessageId) {
	const actorId = ctx.requestToActor.get([messageId.gatewayId, messageId.requestId]);
	if (actorId) {
		let actor = getActor(ctx, actorId);
		if (actor) {
			actor.handle.send({ type: "req-abort", messageId });
		}
	}

	ctx.requestToActor.delete([messageId.gatewayId, messageId.requestId]);
}

function handleWebSocketOpen(ctx: EnvoyContext, messageId: protocol.MessageId, open: protocol.ToEnvoyWebSocketOpen) {
	const actor = getActor(ctx, open.actorId);

	if (!actor) {
		log(ctx.shared)?.warn({
			msg: "received request for unknown actor",
			actorId: open.actorId,
		});

		wsSend(ctx.shared, {
			tag: "ToRivetTunnelMessage",
			val: {
				messageId,
				messageKind: {
					tag: "ToRivetWebSocketClose",
					val: {
						code: 1011,
						reason: "Actor not found",
						hibernate: false,
					},
				}
			}
		});

		return;
	}

	ctx.requestToActor.set([messageId.gatewayId, messageId.requestId], open.actorId);

	actor.handle.send({
		type: "ws-open",
		messageId,
		path: open.path,
		headers: open.headers,
	});
}

function handleWebSocketMessage(ctx: EnvoyContext, messageId: protocol.MessageId, msg: protocol.ToEnvoyWebSocketMessage) {
	const actorId = ctx.requestToActor.get([messageId.gatewayId, messageId.requestId]);
	if (actorId) {
		let actor = getActor(ctx, actorId);
		if (actor) {
			actor.handle.send({ type: "ws-msg", messageId, msg });
		}
	}
}

function handleWebSocketClose(ctx: EnvoyContext, messageId: protocol.MessageId, close: protocol.ToEnvoyWebSocketClose) {
	const actorId = ctx.requestToActor.get([messageId.gatewayId, messageId.requestId]);
	if (actorId) {
		let actor = getActor(ctx, actorId);
		if (actor) {
			actor.handle.send({ type: "ws-close", messageId, close });
		}
	}

	ctx.requestToActor.delete([messageId.gatewayId, messageId.requestId]);
}

export function sendHibernatableWebSocketMessageAck(
	ctx: EnvoyContext,
	gatewayId: protocol.GatewayId,
	requestId: protocol.RequestId,
	envoyMessageIndex: number,
) {
	const actorId = ctx.requestToActor.get([gatewayId, requestId]);
	if (actorId) {
		let actor = getActor(ctx, actorId);
		if (actor) {
			actor.handle.send({ type: "hws-ack", gatewayId, requestId, envoyMessageIndex });
		}
	}
}

export function resendBufferedTunnelMessages(ctx: EnvoyContext) {
	if (ctx.bufferedMessages.length === 0) {
		return;
	}

	log(ctx.shared)?.info({
		msg: "resending buffered tunnel messages",
		count: ctx.bufferedMessages.length,
	});

	const messages = ctx.bufferedMessages;
	ctx.bufferedMessages = [];

	for (const msg of messages) {
		wsSend(
			ctx.shared,
			{
				tag: "ToRivetTunnelMessage",
				val: msg,
			},
		);
	}
}

// NOTE: This is a special response that will cause Guard to retry the request
//
// See should_retry_request_inner
// https://github.com/rivet-dev/rivet/blob/222dae87e3efccaffa2b503de40ecf8afd4e31eb/engine/packages/guard-core/src/proxy_service.rs#L2458
function sendErrorResponse(ctx: EnvoyContext, gatewayId: protocol.GatewayId, requestId: protocol.RequestId) {
	const body = new TextEncoder().encode("Actor not found").buffer;
	const headers = new Map([["x-rivet-error", "envoy.actor_not_found"]]);

	// Add Content-Length header if we have a body and it's not already set
	if (body && !headers.has("content-length")) {
		headers.set("content-length", String(body.byteLength));
	}

	sendMessage(
		ctx,
		gatewayId,
		requestId,
		{
			tag: "ToRivetResponseStart",
			val: {
				status: 503,
				headers,
				body: body,
				stream: false,
			}
		}
	);
}

export async function sendMessage(ctx: EnvoyContext, gatewayId: protocol.GatewayId, requestId: protocol.RequestId, msg: protocol.ToRivetTunnelMessageKind) {
	const payload = {
		messageId: {
			gatewayId,
			requestId,
			messageIndex: 0,
		},
		messageKind: msg,
	};

	const failed = wsSend(
		ctx.shared,
		{
			tag: "ToRivetTunnelMessage",
			val: payload
		},
	);

	// Buffer message if not connected
	if (failed) {
		log(ctx.shared)?.debug({
			msg: "buffering tunnel message, socket not connected to engine",
			requestId: idToStr(requestId),
			message: stringifyToRivetTunnelMessageKind(msg),
		});
		ctx.bufferedMessages.push(payload);
		return;
	}
}
