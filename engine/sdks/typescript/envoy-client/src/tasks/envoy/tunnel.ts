import * as protocol from "@rivetkit/engine-envoy-protocol";
import { EnvoyContext, findActor, log } from "./index.js";
import { SharedContext } from "@/context.js";
import { unreachable } from "antiox";
import { wsSend } from "../connection.js";

export function handleTunnelMessage(ctx: EnvoyContext, msg: protocol.ToEnvoyTunnelMessage) {
	if (msg.messageKind.tag === "ToEnvoyRequestStart") {
		handleRequestStart(ctx, msg.messageId, msg.messageKind.val);
	} else if (msg.messageKind.tag === "ToEnvoyRequestChunk") {
		handleRequestChunk(ctx, msg.messageId, msg.messageKind.val);
	} else if (msg.messageKind.tag === "ToEnvoyRequestAbort") {
		handleRequestAbort(ctx, msg.messageId);
	} else {
		unreachable(msg.messageKind.tag);
	}
}

function handleRequestStart(ctx: EnvoyContext, messageId: protocol.MessageId, req: protocol.ToEnvoyRequestStart) {
	const actor = findActor(ctx, req.actorId);

	if (!actor) {
		log(ctx.shared)?.warn({
			msg: "received request for unknown actor",
			actorId: req.actorId,
		});

		// NOTE: This is a special response that will cause Guard to retry the request
		//
		// See should_retry_request_inner
		// https://github.com/rivet-dev/rivet/blob/222dae87e3efccaffa2b503de40ecf8afd4e31eb/engine/packages/guard-core/src/proxy_service.rs#L2458
		sendResponse(ctx.shared, messageId, new Response("Actor not found", {
			status: 503,
			headers: { "x-rivet-error": "envoy.actor_not_found" },
		}));

		return;
	}

	actor.handle.send({
		type: "request-start",
		messageId,
		req,
	});
}

function handleRequestChunk(ctx: EnvoyContext, messageId: protocol.MessageId, chunk: protocol.ToEnvoyRequestChunk) {
	const actorId = ctx.requestToActor.get([messageId.gatewayId, messageId.requestId]);
	if (actorId) {
		let actor = findActor(ctx, actorId);
		if (actor) {
			actor.handle.send({ type: "request-chunk", messageId, chunk });
		}
	}

	if (chunk.finish) {
		ctx.requestToActor.delete([messageId.gatewayId, messageId.requestId]);
	}
}

function handleRequestAbort(ctx: EnvoyContext, messageId: protocol.MessageId) {
	const actorId = ctx.requestToActor.get([messageId.gatewayId, messageId.requestId]);
	if (actorId) {
		let actor = findActor(ctx, actorId);
		if (actor) {
			actor.handle.send({ type: "request-abort", messageId });
		}
	}

	ctx.requestToActor.delete([messageId.gatewayId, messageId.requestId]);
}

export async function sendResponse(ctx: SharedContext, messageId: protocol.MessageId, response: Response) {
	// Always treat responses as non-streaming for now
	// In the future, we could detect streaming responses based on:
	// - Transfer-Encoding: chunked
	// - Content-Type: text/event-stream
	// - Explicit stream flag from the handler

	// Read the body first to get the actual content
	const body = response.body ? await response.arrayBuffer() : null;

	if (body && body.byteLength > ctx.protocolMetadata?.maxPayloadSize) {
		throw new Error("Response body too large");
	}

	// Convert headers to map and add Content-Length if not present
	const headers = new Map<string, string>();
	response.headers.forEach((value, key) => {
		headers.set(key, value);
	});

	// Add Content-Length header if we have a body and it's not already set
	if (body && !headers.has("content-length")) {
		headers.set("content-length", String(body.byteLength));
	}

	wsSend(
		ctx, {
		tag: "ToRivetTunnelMessage",
		val: {
			messageId,
			messageKind: {
				tag: "ToRivetResponseStart",
				val: {
					status: response.status as protocol.u16,
					headers,
					body: body || null,
					stream: false,
				}
			}
		}
	}
	);

}