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
import { arraysEqual, BufferMap, idToStr, stringifyError } from "../utils.js";
import { HibernatingWebSocketMetadata } from "./envoy/tunnel.js";
import { HIBERNATABLE_SYMBOL, WebSocketTunnelAdapter } from "@/websocket.js";
import { wsSend } from "./connection.js";
import { stringifyToRivetTunnelMessageKind } from "@/stringify.js";

export interface CreateActorOpts {
	actorId: string;
	generation: number;
	config: protocol.ActorConfig;

	hibernatingRequests: readonly protocol.HibernatingRequest[];
}

export type ToActor =
	// Sent when wants to stop the actor, will be forwarded to Envoy
	| {
		type: "intent";
		intent: protocol.ActorIntent;
		error?: string;
	}
	// Sent when actor is told to stop
	| {
		type: "stop";
		commandIdx: bigint;
		reason: protocol.StopActorReason;
	}
	| { type: "lost" }
	// Set or clear an alarm
	| {
		type: "set-alarm";
		alarmTs: bigint | null;
	}
	| {
		type: "req-start";
		messageId: protocol.MessageId;
		req: protocol.ToEnvoyRequestStart;
	}
	| {
		type: "req-chunk";
		messageId: protocol.MessageId;
		chunk: protocol.ToEnvoyRequestChunk;
	}
	| {
		type: "req-abort";
		messageId: protocol.MessageId;
	}
	| {
		type: "ws-open";
		messageId: protocol.MessageId;
		path: string;
		headers: ReadonlyMap<string, string>;
	}
	| {
		type: "ws-msg";
		messageId: protocol.MessageId;
		msg: protocol.ToEnvoyWebSocketMessage;
	}
	| {
		type: "ws-close";
		messageId: protocol.MessageId;
		close: protocol.ToEnvoyWebSocketClose;
	}
	| {
		type: "hws-restore";
		metaEntries: HibernatingWebSocketMetadata[];
	}
	| {
		type: "hws-ack";
		gatewayId: protocol.GatewayId;
		requestId: protocol.RequestId;
		envoyMessageIndex: number;
	};

interface ActorContext {
	shared: SharedContext;
	actorId: string;
	generation: number;
	config: protocol.ActorConfig;
	commandIdx: bigint;
	eventIndex: bigint;
	error?: string;

	// Tunnel requests, not http requests
	pendingRequests: BufferMap<
		PendingRequest
	>;
	webSockets: BufferMap<
		WebSocketTunnelAdapter
	>;
	hibernationRestored: boolean;
	hibernatingRequests: readonly protocol.HibernatingRequest[];
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
		commandIdx: 0n,
		eventIndex: 0n,

		pendingRequests: new BufferMap(),
		webSockets: new BufferMap(),
		hibernationRestored: false,
		hibernatingRequests: opts.hibernatingRequests,
	};

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

		sendEvent(ctx, {
			tag: "EventActorStateUpdate",
			val: {
				state: {
					tag: "ActorStateStopped",
					val: {
						code: protocol.StopCode.Error,
						message: error instanceof Error ? error.message : "actor start failed"
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
		if (msg.type === "intent") {
			sendEvent(ctx, {
				tag: "EventActorIntent",
				val: { intent: msg.intent },
			});
			if (msg.error) ctx.error = msg.error;
		} else if (msg.type === "stop") {
			if (msg.commandIdx <= ctx.commandIdx) {
				log(ctx)?.warn({
					msg: "ignoring already seen command",
					commandIdx: msg.commandIdx
				});
			}
			ctx.commandIdx = msg.commandIdx;

			handleStop(ctx, msg.reason);
			break;
		} else if (msg.type === "lost") {
			handleStop(ctx, protocol.StopActorReason.Lost);
			break;
		} else if (msg.type === "set-alarm") {
			sendEvent(ctx, {
				tag: "EventActorSetAlarm",
				val: { alarmTs: msg.alarmTs },
			});
		} else if (msg.type === "req-start") {
			handleReqStart(ctx, msg.messageId, msg.req);
		} else if (msg.type === "req-chunk") {
			handleReqChunk(ctx, msg.messageId, msg.chunk);
		} else if (msg.type === "req-abort") {
			handleReqAbort(ctx, msg.messageId);
		} else if (msg.type === "ws-open") {
			handleWsOpen(ctx, msg.messageId, msg.path, msg.headers);
		} else if (msg.type === "ws-msg") {
			handleWsMessage(ctx, msg.messageId, msg.msg);
		} else if (msg.type === "ws-close") {
			handleWsClose(ctx, msg.messageId, msg.close);
		} else if (msg.type === "hws-restore") {
			handleHwsRestore(ctx, msg.metaEntries);
		} else if (msg.type === "hws-ack") {
			handleHwsAck(ctx, msg.gatewayId, msg.requestId, msg.envoyMessageIndex);
		} else {
			unreachable(msg);
		}
	}

	log(ctx)?.debug({
		msg: "envoy actor stopped"
	});

	rx.close();
}

interface PendingRequest {
	envoyMessageIndex: number;
	streamController?: ReadableStreamDefaultController<Uint8Array>;
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

async function handleStop(ctx: ActorContext, reason: protocol.StopActorReason) {
	let stopCode = ctx.error ? protocol.StopCode.Error : protocol.StopCode.Ok;
	let stopMessage: string | null = ctx.error ?? null;

	try {
		await ctx.shared.config.onActorStop(
			ctx.shared.handle,
			ctx.actorId,
			ctx.generation,
			reason,
		);
	} catch (error) {
		log(ctx)?.error({
			msg: "actor stop failed",
			actorId: ctx.actorId,
			error: stringifyError(error),
		});

		stopCode = protocol.StopCode.Error;
		if (!stopMessage) {
			stopMessage =
				error instanceof Error
					? error.message
					: "actor stop failed";
		}
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
}

function handleReqStart(ctx: ActorContext, messageId: protocol.MessageId, req: protocol.ToEnvoyRequestStart) {
	let pendingReq: PendingRequest = {
		envoyMessageIndex: 0,
	};
	ctx.pendingRequests.set(
		[messageId.gatewayId, messageId.requestId],
		pendingReq,
	);

	// Convert headers map to Headers object
	const headers = new Headers();
	for (const [key, value] of req.headers) {
		headers.append(key, value);
	}

	// Create Request object
	const request = new Request(`http://localhost${req.path}`, {
		method: req.method,
		headers,
		body: req.body ? new Uint8Array(req.body) : undefined,
	});

	// Handle streaming request
	if (req.stream) {
		// Create a stream for the request body
		const stream = new ReadableStream<Uint8Array>({
			start: (controller) => {
				// Store controller for chunks
				pendingReq.streamController = controller;
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
				messageId.gatewayId,
				messageId.requestId,
				streamingRequest,
			);
			await sendResponse(
				ctx,
				messageId.gatewayId,
				messageId.requestId,
				response,
			);
		});
	} else {
		// Non-streaming request
		spawn(async () => {
			const response = await ctx.shared.config.fetch(
				ctx.shared.handle,
				ctx.actorId,
				messageId.gatewayId,
				messageId.requestId,
				request,
			);
			await sendResponse(
				ctx,
				messageId.gatewayId,
				messageId.requestId,
				response,
			);
			ctx.pendingRequests.delete(
				[messageId.gatewayId, messageId.requestId],
			);
		});
	}
}

function handleReqChunk(ctx: ActorContext, messageId: protocol.MessageId, chunk: protocol.ToEnvoyRequestChunk) {
	const req = ctx.pendingRequests.get(
		[messageId.gatewayId, messageId.requestId]
	);
	if (req) {
		if (req.streamController) {
			req.streamController.enqueue(new Uint8Array(chunk.body));

			if (chunk.finish) {
				req.streamController.close();

				ctx.pendingRequests.delete(
					[messageId.gatewayId, messageId.requestId],
				);
			}
		} else {
			log(ctx)?.warn({
				msg: "received chunk for pending request without stream controller",
			});

		}
	} else {
		log(ctx)?.warn({
			msg: "received chunk for unknown pending request",
		});
	}
}

function handleReqAbort(ctx: ActorContext, messageId: protocol.MessageId) {
	const req = ctx.pendingRequests.get(
		[messageId.gatewayId, messageId.requestId]
	);
	if (req) {
		if (req.streamController) {
			req.streamController.error(new Error("Request aborted"));
		}

		ctx.pendingRequests.delete(
			[messageId.gatewayId, messageId.requestId],
		);
	} else {
		log(ctx)?.warn({
			msg: "received abort for unknown pending request",
		});
	}
}

async function handleWsOpen(ctx: ActorContext, messageId: protocol.MessageId, path: string, headers: ReadonlyMap<string, string>) {
	ctx.pendingRequests.set(
		[messageId.gatewayId, messageId.requestId],
		{
			envoyMessageIndex: 0,
		}
	);

	try {
		// #createWebSocket will call `runner.config.websocket` under the
		// hood to add the event listeners for open, etc. If this handler
		// throws, then the WebSocket will be closed before sending the
		// open event.
		const adapter = await createWebSocket(
			ctx,
			messageId,
			false,
			path,
			Object.fromEntries(headers),
		);
		ctx.webSockets.set([messageId.gatewayId, messageId.requestId], adapter);

		sendMessage(ctx, messageId.gatewayId, messageId.requestId, {
			tag: "ToRivetWebSocketOpen",
			val: {
				canHibernate: adapter[HIBERNATABLE_SYMBOL],
			},
		});

		adapter._handleOpen();
	} catch (error) {
		log(ctx)?.error({ msg: "error handling websocket open", error });

		// Send close on error
		sendMessage(ctx, messageId.gatewayId, messageId.requestId, {
			tag: "ToRivetWebSocketClose",
			val: {
				code: 1011,
				reason: "Server Error",
				hibernate: false,
			},
		});

		ctx.pendingRequests.delete([messageId.gatewayId, messageId.requestId]);
		ctx.webSockets.delete([messageId.gatewayId, messageId.requestId]);
	}
}

function handleWsMessage(ctx: ActorContext, messageId: protocol.MessageId, msg: protocol.ToEnvoyWebSocketMessage) {
	const ws = ctx.webSockets.get(
		[messageId.gatewayId, messageId.requestId]
	);
	if (ws) {
		const data = msg.binary
			? new Uint8Array(msg.data)
			: new TextDecoder().decode(new Uint8Array(msg.data));

		ws._handleMessage(
			data,
			messageId.messageIndex,
			msg.binary,
		);
	} else {
		log(ctx)?.warn({
			msg: "received message for unknown ws",
		});
	}
}

function handleWsClose(ctx: ActorContext, messageId: protocol.MessageId, close: protocol.ToEnvoyWebSocketClose) {
	const ws = ctx.webSockets.get(
		[messageId.gatewayId, messageId.requestId]
	);
	if (ws) {
		// We don't need to send a close response
		ws._handleClose(
			close.code || undefined,
			close.reason || undefined,
		);
		ctx.webSockets.delete(
			[messageId.gatewayId, messageId.requestId]
		);
		ctx.pendingRequests.delete(
			[messageId.gatewayId, messageId.requestId]
		);
	} else {
		log(ctx)?.warn({
			msg: "received close for unknown ws",
		});
	}
}

async function handleHwsRestore(ctx: ActorContext, metaEntries: HibernatingWebSocketMetadata[]) {
	if (ctx.hibernationRestored) {
		throw new Error(
			`Actor ${ctx.actorId} already restored hibernating requests`,
		);
	}

	log(ctx)?.debug({
		msg: "restoring hibernating requests",
		requests: ctx.hibernatingRequests.length,
	});

	// Track all background operations
	const backgroundOperations: Promise<void>[] = [];

	// Process connected WebSockets
	let connectedButNotLoadedCount = 0;
	let restoredCount = 0;
	for (const { gatewayId, requestId } of ctx.hibernatingRequests) {
		const requestIdStr = idToStr(requestId);
		const meta = metaEntries.find(
			(entry) =>
				arraysEqual(entry.gatewayId, gatewayId) &&
				arraysEqual(entry.requestId, requestId),
		);

		if (!meta) {
			// Connected but not loaded (not persisted) - close it
			//
			// This may happen if the metadata was not successfully persisted
			log(ctx)?.warn({
				msg: "closing websocket that is not persisted",
				requestId: requestIdStr,
			});

			sendMessage(ctx, gatewayId, requestId, {
				tag: "ToRivetWebSocketClose",
				val: {
					code: 1000,
					reason: "ws.meta_not_found_during_restore",
					hibernate: false,
				},
			});

			connectedButNotLoadedCount++;
		} else {
			ctx.pendingRequests.set([gatewayId, requestId], { envoyMessageIndex: 0 });

			// This will call `runner.config.websocket` under the hood to
			// attach the event listeners to the WebSocket.
			// Track this operation to ensure it completes
			const restoreOperation = createWebSocket(
				ctx,
				{
					gatewayId,
					requestId,
					messageIndex: meta.rivetMessageIndex,
				},
				true,
				meta.path,
				meta.headers,
			)
				.then(adapter => {
					ctx.webSockets.set([gatewayId, requestId], adapter);

					log(ctx)?.info({
						msg: "connection successfully restored",
						requestId: requestIdStr,
					});
				})
				.catch((err) => {
					log(ctx)?.error({
						msg: "error creating websocket during restore",
						requestId: requestIdStr,
						error: stringifyError(err),
					});

					// Close the WebSocket on error
					sendMessage(ctx, gatewayId, requestId, {
						tag: "ToRivetWebSocketClose",
						val: {
							code: 1011,
							reason: "ws.restore_error",
							hibernate: false,
						},
					});

					ctx.pendingRequests.delete([gatewayId, requestId]);
				});

			backgroundOperations.push(restoreOperation);
			restoredCount++;
		}
	}

	// Process loaded but not connected (stale) - remove them
	let loadedButNotConnectedCount = 0;
	for (const meta of metaEntries) {
		const requestIdStr = idToStr(meta.requestId);
		const isConnected = ctx.hibernatingRequests.some(
			(req) =>
				arraysEqual(req.gatewayId, meta.gatewayId) &&
				arraysEqual(req.requestId, meta.requestId),
		);
		if (!isConnected) {
			log(ctx)?.warn({
				msg: "removing stale persisted websocket",
				requestId: requestIdStr,
			});

			// Create adapter to register user's event listeners.
			// Pass engineAlreadyClosed=true so close callback won't send tunnel message.
			// Track this operation to ensure it completes
			const cleanupOperation = createWebSocket(
				ctx,
				{
					gatewayId: meta.gatewayId,
					requestId: meta.requestId,
					messageIndex: meta.rivetMessageIndex,
				},
				true,
				meta.path,
				meta.headers,
			)
				.then((adapter) => {
					// Close the adapter normally - this will fire user's close event handler
					// (which should clean up persistence) and trigger the close callback
					// (which will clean up maps but skip sending tunnel message)
					adapter.close(1000, "ws.stale_metadata");
				})
				.catch((err) => {
					log(ctx)?.error({
						msg: "error creating stale websocket during restore",
						requestId: requestIdStr,
						error: stringifyError(err),
					});
				});

			backgroundOperations.push(cleanupOperation);
			loadedButNotConnectedCount++;
		}
	}

	// Wait for all background operations to complete before finishing
	await Promise.allSettled(backgroundOperations);

	// Mark restoration as complete
	ctx.hibernationRestored = true;

	log(ctx)?.info({
		msg: "restored hibernatable websockets",
		restoredCount,
		connectedButNotLoadedCount,
		loadedButNotConnectedCount,
	});
}

function handleHwsAck(ctx: ActorContext, gatewayId: protocol.GatewayId, requestId: protocol.RequestId, envoyMessageIndex: number) {
	const requestIdStr = idToStr(requestId);

	log(ctx)?.debug({
		msg: "ack ws msg",
		requestId: requestIdStr,
		index: envoyMessageIndex,
	});

	if (envoyMessageIndex < 0 || envoyMessageIndex > 65535)
		throw new Error("Invalid websocket ack index");

	// Send the ack message
	sendMessage(ctx, gatewayId, requestId, {
		tag: "ToRivetWebSocketMessageAck",
		val: {
			index: envoyMessageIndex,
		},
	});
}

function incrementCheckpoint(ctx: ActorContext): protocol.ActorCheckpoint {
	const index = ctx.eventIndex;
	ctx.eventIndex++;

	return { actorId: ctx.actorId, generation: ctx.generation, index };
}

async function createWebSocket(
	ctx: ActorContext,
	messageId: protocol.MessageId,
	isRestoringHibernatable: boolean,
	path: string,
	headers: Record<string, string>,
): Promise<WebSocketTunnelAdapter> {
	// We need to manually ensure the original Upgrade/Connection WS
	// headers are present
	const fullHeaders = {
		...headers,
		Upgrade: "websocket",
		Connection: "Upgrade",
	};

	if (!path.startsWith("/")) {
		throw new Error("Path must start with leading slash");
	}

	const request = new Request(`http://actor${path}`, {
		method: "GET",
		headers: fullHeaders,
	});

	const isHibernatable = isRestoringHibernatable ||
		ctx.shared.config.hibernatableWebSocket.canHibernate(
			ctx.actorId,
			messageId.gatewayId,
			messageId.requestId,
			request,
		);

	// Create WebSocket adapter
	const adapter = new WebSocketTunnelAdapter(
		ctx.shared,
		ctx.actorId,
		messageId.gatewayId,
		messageId.requestId,
		messageId.messageIndex,
		isHibernatable,
		isRestoringHibernatable,
		request,
		(data: ArrayBuffer | string, isBinary: boolean) => {
			// Send message through tunnel
			const dataBuffer =
				typeof data === "string"
					? (new TextEncoder().encode(data).buffer as ArrayBuffer)
					: data;

			sendMessage(ctx, messageId.gatewayId, messageId.requestId, {
				tag: "ToRivetWebSocketMessage",
				val: {
					data: dataBuffer,
					binary: isBinary,
				},
			});
		},
		(code?: number, reason?: string) => {
			sendMessage(ctx, messageId.gatewayId, messageId.requestId, {
				tag: "ToRivetWebSocketClose",
				val: {
					code: code || null,
					reason: reason || null,
					hibernate: false,
				},
			});

			ctx.pendingRequests.delete([messageId.gatewayId, messageId.requestId]);
			ctx.webSockets.delete([messageId.gatewayId, messageId.requestId]);
		},
	);

	// Call WebSocket handler. This handler will add event listeners
	// for `open`, etc. Pass the VirtualWebSocket (not the adapter) to the actor.
	await ctx.shared.config.websocket(
		ctx.shared.handle,
		ctx.actorId,
		adapter.websocket,
		messageId.gatewayId,
		messageId.requestId,
		request,
		path,
		headers,
		isHibernatable,
		isRestoringHibernatable,
	);

	return adapter;
}

async function sendResponse(ctx: ActorContext, gatewayId: protocol.GatewayId, requestId: protocol.RequestId, response: Response) {
	// Always treat responses as non-streaming for now
	// In the future, we could detect streaming responses based on:
	// - Transfer-Encoding: chunked
	// - Content-Type: tbackgroundOperationsext/event-stream
	// - Explicit stream flag from the handler

	// Read the body first to get the actual content
	const body = response.body ? await response.arrayBuffer() : null;

	if (body && body.byteLength > (ctx.shared.protocolMetadata?.maxResponsePayloadSize ?? Infinity)) {
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

	sendMessage(
		ctx,
		gatewayId,
		requestId,
		{
			tag: "ToRivetResponseStart",
			val: {
				status: response.status as protocol.u16,
				headers,
				body: body || null,
				stream: false,
			}
		}
	);
}

export async function sendMessage(
	ctx: ActorContext,
	gatewayId: protocol.GatewayId,
	requestId: protocol.RequestId,
	messageKind: protocol.ToRivetTunnelMessageKind,
) {
	const gatewayIdStr = idToStr(gatewayId);
	const requestIdStr = idToStr(requestId);

	// Get message index from pending request
	const req = ctx.pendingRequests.get([gatewayId, requestId]);
	if (!req) {
		// No pending request
		log(ctx)?.warn({
			msg: "missing pending request for send message",
			gatewayId: gatewayIdStr,
			requestId: requestIdStr,
		});
		return;
	}

	const envoyMessageIndex = req.envoyMessageIndex;
	req.envoyMessageIndex++;

	const msg = {
		messageId: {
			gatewayId,
			requestId,
			messageIndex: envoyMessageIndex,
		},
		messageKind,
	};

	const failed = wsSend(
		ctx.shared,
		{
			tag: "ToRivetTunnelMessage",
			val: msg,
		},
	);

	// Buffer message if not connected
	if (failed) {
		log(ctx)?.debug({
			msg: "buffering tunnel message, socket not connected to engine",
			requestId: idToStr(requestId),
			message: stringifyToRivetTunnelMessageKind(msg.messageKind),
		});
		ctx.shared.envoyTx.send({ type: "buffer-tunnel-msg", msg });
		return;
	}
}

function log(ctx: ActorContext) {
	const baseLogger = ctx.shared.config.logger ?? logger();
	if (!baseLogger) return undefined;

	return baseLogger.child({
		actorId: ctx.actorId,
		generation: ctx.generation,
	});
}
