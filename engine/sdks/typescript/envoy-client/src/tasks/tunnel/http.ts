import type * as protocol from "@rivetkit/engine-envoy-protocol";
import { spawn } from "antiox/task";
import type { EnvoyContext } from "../envoy/index.js";
import { findActiveActor, log } from "../envoy/index.js";
import { MAX_PAYLOAD_SIZE, stringifyError } from "../../utils.js";
import {
	type RequestEntry,
	requestKey,
	sendTunnelMessage,
	sendTunnelMessageRaw,
} from "./index.js";

export function handleRequestStart(
	ctx: EnvoyContext,
	gatewayId: protocol.GatewayId,
	requestId: protocol.RequestId,
	req: protocol.ToEnvoyRequestStart,
): void {
	const key = requestKey(gatewayId, requestId);

	const actor = findActiveActor(ctx, req.actorId);
	if (!actor) {
		log(ctx.shared)?.warn({
			msg: "ignoring request for unknown actor",
			actorId: req.actorId,
		});
		sendTunnelMessageRaw(ctx, gatewayId, requestId, {
			tag: "ToRivetResponseStart",
			val: {
				status: 503,
				headers: new Map([
					["content-type", "text/plain"],
					["x-rivet-error", "envoy.actor_not_found"],
				]),
				body: new TextEncoder().encode("Actor not found")
					.buffer as ArrayBuffer,
				stream: false,
			},
		});
		return;
	}

	// Build request headers
	const headers = new Headers();
	for (const [k, v] of req.headers) {
		headers.append(k, v);
	}

	// Create entry synchronously before spawning the fetch task
	const entry: RequestEntry = {
		actorId: req.actorId,
		generation: actor.generation,
		gatewayId,
		requestId,
		clientMessageIndex: 0,
	};
	ctx.tunnelRequests.set(key, entry);

	let request: Request;
	if (req.stream) {
		const stream = new ReadableStream<Uint8Array>({
			start: (controller) => {
				entry.streamController = controller;
				if (req.body) {
					controller.enqueue(new Uint8Array(req.body));
				}
			},
		});

		request = new Request(`http://localhost${req.path}`, {
			method: req.method,
			headers,
			body: stream,
			duplex: "half",
		} as any);
	} else {
		request = new Request(`http://localhost${req.path}`, {
			method: req.method,
			headers,
			body: req.body ? new Uint8Array(req.body) : undefined,
		});
	}

	const capturedEntry = entry;
	const actorStartPromise = actor.entry.actorStartPromise;
	const { actorId } = req;

	spawn(async () => {
		try {
			await actorStartPromise;

			if (ctx.shuttingDown) return;

			const response = await ctx.shared.config.fetch(
				ctx.shared.handle,
				actorId,
				gatewayId,
				requestId,
				request,
			);

			// Staleness check before sending response
			if (ctx.tunnelRequests.get(key) !== capturedEntry) return;

			await sendResponse(ctx, gatewayId, requestId, response);
		} catch (error) {
			if (ctx.tunnelRequests.get(key) !== capturedEntry) return;

			log(ctx.shared)?.error({
				msg: "error handling request",
				actorId,
				error: stringifyError(error),
			});

			sendResponseError(
				ctx,
				gatewayId,
				requestId,
				500,
				"Internal Server Error",
			);
		} finally {
			if (ctx.tunnelRequests.get(key) === capturedEntry) {
				ctx.tunnelRequests.delete(key);
			}
		}
	});
}

export function handleRequestChunk(
	ctx: EnvoyContext,
	gatewayId: protocol.GatewayId,
	requestId: protocol.RequestId,
	chunk: protocol.ToEnvoyRequestChunk,
): void {
	const key = requestKey(gatewayId, requestId);
	const entry = ctx.tunnelRequests.get(key);

	if (!entry) {
		log(ctx.shared)?.warn({
			msg: "received request chunk for unknown request",
			requestKey: key,
		});
		return;
	}

	if (!entry.streamController) {
		log(ctx.shared)?.warn({
			msg: "received request chunk for non-streaming request",
			requestKey: key,
		});
		return;
	}

	try {
		entry.streamController.enqueue(new Uint8Array(chunk.body));
		if (chunk.finish) {
			entry.streamController.close();
		}
	} catch (error) {
		log(ctx.shared)?.warn({
			msg: "error enqueuing chunk",
			requestKey: key,
			error: stringifyError(error),
		});
	}
}

export function handleRequestAbort(
	ctx: EnvoyContext,
	gatewayId: protocol.GatewayId,
	requestId: protocol.RequestId,
): void {
	const key = requestKey(gatewayId, requestId);
	const entry = ctx.tunnelRequests.get(key);

	if (!entry) {
		log(ctx.shared)?.warn({
			msg: "received request abort for unknown request",
			requestKey: key,
		});
		return;
	}

	if (entry.streamController) {
		try {
			entry.streamController.error(new Error("Request aborted"));
		} catch {
			// Controller may already be closed
		}
	}

	ctx.tunnelRequests.delete(key);
}

async function sendResponse(
	ctx: EnvoyContext,
	gatewayId: protocol.GatewayId,
	requestId: protocol.RequestId,
	response: Response,
): Promise<void> {
	const body = response.body ? await response.arrayBuffer() : null;

	if (body && body.byteLength > MAX_PAYLOAD_SIZE) {
		sendResponseError(
			ctx,
			gatewayId,
			requestId,
			500,
			"Response body too large",
		);
		return;
	}

	const headers = new Map<string, string>();
	response.headers.forEach((value, key) => {
		headers.set(key, value);
	});

	if (body && !headers.has("content-length")) {
		headers.set("content-length", String(body.byteLength));
	}

	sendTunnelMessage(ctx, gatewayId, requestId, {
		tag: "ToRivetResponseStart",
		val: {
			status: response.status as protocol.u16,
			headers,
			body: body ?? null,
			stream: false,
		},
	});
}

function sendResponseError(
	ctx: EnvoyContext,
	gatewayId: protocol.GatewayId,
	requestId: protocol.RequestId,
	status: number,
	message: string,
): void {
	sendTunnelMessage(ctx, gatewayId, requestId, {
		tag: "ToRivetResponseStart",
		val: {
			status: status as protocol.u16,
			headers: new Map([["content-type", "text/plain"]]),
			body: new TextEncoder().encode(message).buffer as ArrayBuffer,
			stream: false,
		},
	});
}
