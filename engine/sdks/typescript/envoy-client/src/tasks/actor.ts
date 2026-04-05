import * as protocol from "@rivetkit/engine-envoy-protocol";
import {
	type UnboundedReceiver,
	type UnboundedSender,
	unboundedChannel,
} from "antiox/sync/mpsc";
import { spawn } from "antiox/task";
import type { SharedContext } from "../context.js";
import { logger } from "../log.js";
import { unreachable } from "antiox/panic";
import { stringifyError } from "../utils.js";
import { sendResponse } from "./envoy/tunnel.js";

export interface CreateActorOpts {
	commandIdx: bigint;
	actorId: string;
	generation: number;
	config: protocol.ActorConfig;
	hibernatingRequests: readonly protocol.HibernatingRequest[];
}

/**
 *
 * Stop sequence:
 * 1. X -> Actor: stop-intent (optional)
 * 1. Actor -> Envoy: send-events (optional)
 * 1. Envoy -> Actor: command-stop-actor
 * 1. Actor: async cleanup
 * 1. Actor -> Envoy: state update (stopped)
 */

// TODO: envoy lost
export type ToActor =
	// Sent when wants to stop the actor, will be forwarded to Envoy
	| {
		type: "actor-intent";
		commandIdx: bigint;
		intent: protocol.ActorIntent;
	}
	// Sent when actor is told to stop
	| {
		type: "command-stop-actor";
		commandIdx: bigint;
		reason: protocol.StopActorReason;
	}
	// Set or clear an alarm
	| {
		type: "set-alarm";
		alarmTs: bigint | null;
	}
	| {
		type: "request-start";
		messageId: protocol.MessageId,
		req: protocol.ToEnvoyRequestStart,
	}
	| {
		type: "request-chunk";
		messageId: protocol.MessageId,
		chunk: protocol.ToEnvoyRequestChunk;
	} | {
		type: "request-abort";
		messageId: protocol.MessageId,
	};

interface ActorContext {
	shared: SharedContext;
	actorId: string;
	generation: number;
	config: protocol.ActorConfig;
	eventIndex: bigint;
	pendingRequests: Map<
		[protocol.GatewayId, protocol.RequestId],
		PendingRequest
	>;
	webSockets: Map<
		[protocol.GatewayId, protocol.RequestId],
		WebSocketTunnelAdapter
	>;
}

export function createActor(
	ctx: SharedContext,
	start: CreateActorOpts,
): UnboundedSender<ToActor> {
	const [tx, rx] = unboundedChannel<ToActor>();
	spawn(() => actorInner(ctx, start, rx));
	return tx;
}

async function actorInner(
	shared: SharedContext,
	opts: CreateActorOpts,
	rx: UnboundedReceiver<ToActor>,
) {
	const ctx: ActorContext = {
		shared,
		actorId: opts.actorId,
		generation: opts.generation,
		config: opts.config,
		eventIndex: 0n,
		pendingRequests: new Map(),
		// webSockets: new Map(),
	};

	let stopCode = protocol.StopCode.Ok;
	let stopMessage: string | null = null;

	try {
		await shared.config.onActorStart(
			shared.handle,
			opts.actorId,
			opts.generation,
			opts.config,
		);
	} catch (error) {
		log(ctx)?.error({
			msg: "actor start failed",
			actorId: opts.actorId,
			error: stringifyError(error),
		});

		stopCode = protocol.StopCode.Error;
		stopMessage =
			error instanceof Error ? error.message : "actor start failed";

		sendEvent(ctx, {
			tag: "EventActorStateUpdate",
			val: {
				state: {
					tag: "ActorStateStopped",
					val: {
						code: stopCode,
						message: stopMessage
					},
				},
			},
		});
		return;
	}

	sendEvent(ctx, {
		tag: "EventActorStateUpdate",
		val: { state: { tag: "ActorStateRunning", val: null } },
	});

	for await (const msg of rx) {
		if (msg.type === "actor-intent") {
			sendEvent(ctx, {
				tag: "EventActorIntent",
				val: { intent: msg.intent },
			});
		} else if (msg.type === "command-stop-actor") {
			try {
				await ctx.shared.config.onActorStop(
					ctx.shared.handle,
					ctx.actorId,
					ctx.generation,
					msg.reason,
				);
			} catch (error) {
				log(ctx)?.error({
					msg: "actor stop failed",
					actorId: ctx.actorId,
					error: stringifyError(error),
				});

				stopCode = protocol.StopCode.Error;
				stopMessage =
					error instanceof Error
						? error.message
						: "actor stop failed";
			}

			sendEvent(ctx, {
				tag: "EventActorStateUpdate",
				val: {
					state: {
						tag: "ActorStateStopped",
						val: {
							code: stopCode,
							message: stopMessage
						},
					},
				},
			});
			return;
		} else if (msg.type === "set-alarm") {
			sendEvent(ctx, {
				tag: "EventActorSetAlarm",
				val: { alarmTs: msg.alarmTs },
			});
		} else if (msg.type === "request-start") {
			// Convert headers map to Headers object
			const headers = new Headers();
			for (const [key, value] of msg.req.headers) {
				headers.append(key, value);
			}

			// Create Request object
			const request = new Request(`http://localhost${msg.req.path}`, {
				method: msg.req.method,
				headers,
				body: msg.req.body ? new Uint8Array(msg.req.body) : undefined,
			});

			// Handle streaming request
			if (msg.req.stream) {
				// Create a stream for the request body
				const stream = new ReadableStream<Uint8Array>({
					start: (controller) => {
						// Store controller for chunks
						ctx.pendingRequests.set(
							[msg.messageId.gatewayId, msg.messageId.requestId],
							{
								clientMessageIndex: 0,
								streamController: controller,
							}
						);
					},
				});

				// Create request with streaming body
				const streamingRequest = new Request(request, {
					body: stream,
					duplex: "half",
				} as any);

				spawn(async () => {
					const response = await ctx.shared.config.fetch(
						ctx.shared.handle,
						ctx.actorId,
						msg.messageId.gatewayId,
						msg.messageId.requestId,
						streamingRequest,
					);
					await sendResponse(
						ctx.shared,
						{
							gatewayId: msg.messageId.gatewayId,
							requestId: msg.messageId.requestId,
							messageIndex: 0,
						},
						response,
					);
				});
			} else {
				// Non-streaming request
				spawn(async () => {
					const response = await ctx.shared.config.fetch(
						ctx.shared.handle,
						ctx.actorId,
						msg.messageId.gatewayId,
						msg.messageId.requestId,
						request,
					);
					await sendResponse(
						ctx.shared,
						{
							gatewayId: msg.messageId.gatewayId,
							requestId: msg.messageId.requestId,
							messageIndex: 0,
						},
						response,
					);
				});
			}
		} else if (msg.type === "request-chunk") {
			const existing = ctx.pendingRequests.get(
				[msg.messageId.gatewayId, msg.messageId.requestId]
			);
			if (existing) {
				existing.streamController.enqueue(new Uint8Array(msg.chunk.body));

				if (msg.chunk.finish) {
					existing.streamController.close();

					ctx.pendingRequests.delete(
						[msg.messageId.gatewayId, msg.messageId.requestId],
					);
				}
			} else {
				log(ctx)?.warn({
					msg: "received chunk for unknown pending request",
				});
			}
		} else if (msg.type === "request-abort") {
			const existing = ctx.pendingRequests.get(
				[msg.messageId.gatewayId, msg.messageId.requestId]
			);
			if (existing) {
				existing.streamController.error(new Error("Request aborted"));

				ctx.pendingRequests.delete(
					[msg.messageId.gatewayId, msg.messageId.requestId],
				);
			} else {
				log(ctx)?.warn({
					msg: "received abort for unknown pending request",
				});
			}
		} else {
			unreachable(msg);
		}
	}
}

interface PendingRequest {
	clientMessageIndex: number;
	streamController: ReadableStreamDefaultController<Uint8Array>;
}

function sendEvent(ctx: ActorContext, inner: protocol.Event) {
	ctx.shared.envoyTx.send({
		type: "send-events",
		events: [
			{
				checkpoint: incrementCheckpoint(ctx),
				inner,
			},
		],
	});
}

function incrementCheckpoint(ctx: ActorContext): protocol.ActorCheckpoint {
	const index = ctx.eventIndex;
	ctx.eventIndex++;

	return { actorId: ctx.actorId, generation: ctx.generation, index };
}

function log(ctx: ActorContext) {
	const baseLogger = ctx.shared.config.logger ?? logger();
	if (!baseLogger) return undefined;

	return baseLogger.child({
		actorId: ctx.actorId,
		generation: ctx.generation,
	});
}
