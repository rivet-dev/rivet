import type * as protocol from "@rivetkit/engine-envoy-protocol";
import type { EnvoyContext, ToEnvoyMessage } from "./index.js";
import { log } from "./index.js";
import { stringifyError } from "../../utils.js";
import { wsSend } from "../connection.js";

export interface KvRequestEntry {
	actorId: string;
	data: protocol.KvRequestData;
	resolve: (data: protocol.KvResponseData) => void;
	reject: (error: Error) => void;
	sent: boolean;
	timestamp: number;
}

export const KV_EXPIRE_MS = 30_000;
export const KV_CLEANUP_INTERVAL_MS = 15_000;

export function handleKvRequest(
	ctx: EnvoyContext,
	msg: Extract<ToEnvoyMessage, { type: "kv-request" }>,
) {
	const requestId = ctx.nextKvRequestId++;

	const entry: KvRequestEntry = {
		actorId: msg.actorId,
		data: msg.data,
		resolve: msg.resolve,
		reject: msg.reject,
		sent: false,
		timestamp: Date.now(),
	};

	ctx.kvRequests.set(requestId, entry);

	if (ctx.shared.wsTx) {
		sendSingleKvRequest(ctx, requestId);
	}
}

export function handleKvResponse(
	ctx: EnvoyContext,
	response: protocol.ToEnvoyKvResponse,
) {
	const request = ctx.kvRequests.get(response.requestId);

	if (!request) {
		log(ctx.shared)?.error({
			msg: "received kv response for unknown request id",
			requestId: response.requestId,
		});
		return;
	}

	ctx.kvRequests.delete(response.requestId);

	if (response.data.tag === "KvErrorResponse") {
		request.reject(
			new Error(response.data.val.message || "unknown KV error"),
		);
	} else {
		request.resolve(response.data);
	}
}

export function sendSingleKvRequest(ctx: EnvoyContext, requestId: number) {
	const request = ctx.kvRequests.get(requestId);
	if (!request || request.sent) return;

	try {
		wsSend(ctx.shared, {
			tag: "ToRivetKvRequest",
			val: {
				actorId: request.actorId,
				requestId,
				data: request.data,
			},
		});

		request.sent = true;
		request.timestamp = Date.now();
	} catch (error) {
		ctx.kvRequests.delete(requestId);
		request.reject(
			error instanceof Error ? error : new Error(stringifyError(error)),
		);
	}
}

export function processUnsentKvRequests(ctx: EnvoyContext) {
	if (!ctx.shared.wsTx) return;

	for (const [requestId, request] of ctx.kvRequests) {
		if (!request.sent) {
			sendSingleKvRequest(ctx, requestId);
		}
	}
}

export function cleanupOldKvRequests(ctx: EnvoyContext) {
	const expiry = Date.now() - KV_EXPIRE_MS;
	const toDelete: number[] = [];

	for (const [requestId, request] of ctx.kvRequests) {
		if (request.timestamp < expiry) {
			request.reject(new Error("KV request timed out"));
			toDelete.push(requestId);
		}
	}

	for (const requestId of toDelete) {
		ctx.kvRequests.delete(requestId);
	}
}
