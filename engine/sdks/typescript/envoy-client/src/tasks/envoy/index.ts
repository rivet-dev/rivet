import * as protocol from "@rivetkit/engine-envoy-protocol";
import type { UnboundedSender } from "antiox/sync/mpsc";
import { unboundedChannel } from "antiox/sync/mpsc";
import { v4 as uuidv4 } from "uuid";
import type { ToActor } from "../actor.js";
import type { EnvoyConfig } from "../../config.js";
import type { EnvoyHandle, KvListOptions } from "../../handle.js";
import { startConnection, wsSend } from "../connection.js";
import type { SharedContext } from "../../context.js";
import { logger } from "../../log.js";
import { unreachable } from "antiox/panic";
import {
	ACK_COMMANDS_INTERVAL_MS,
	handleCommands,
	sendCommandAck,
} from "./commands.js";
import {
	handleAckEvents,
	handleSendEvents,
	resendUnacknowledgedEvents,
} from "./events.js";
import { handleTunnelMessage, HibernatingWebSocketMetadata, resendBufferedTunnelMessages, sendHibernatableWebSocketMessageAck } from './tunnel.js';
import {
	KV_CLEANUP_INTERVAL_MS,
	type KvRequestEntry,
	cleanupOldKvRequests,
	handleKvRequest,
	handleKvResponse,
	processUnsentKvRequests,
} from "./kv.js";
import { sleep, spawn, watch, WatchReceiver, WatchSender } from "antiox";
import { BufferMap, EnvoyShutdownError } from "@/utils.js";
import { stringifyToEnvoy } from "@/stringify.js";

export interface EnvoyContext {
	shared: SharedContext;
	serverless: boolean;
	shuttingDown: boolean;
	actors: Map<string, Map<number, ActorEntry>>;
	kvRequests: Map<number, KvRequestEntry>;
	nextKvRequestId: number;
	// Maps tunnel requests to actors (not http requests)
	requestToActor: BufferMap<string>;
	bufferedMessages: protocol.ToRivetTunnelMessage[];
}

export interface ActorEntry {
	handle: UnboundedSender<ToActor>;
	name: string;
	eventHistory: protocol.EventWrapper[];
	lastCommandIdx: bigint;
}

/**
 * Message coming from the connection.
 *
 * Ping is handled by the connection task.
 */
export type ToEnvoyFromConnMessage = Exclude<
	protocol.ToEnvoy,
	{ tag: "ToEnvoyPing" }
>;

export type ToEnvoyMessage =
	// Inbound from connection
	| { type: "conn-message"; message: ToEnvoyFromConnMessage }
	| {
		type: "conn-close";
		evict: boolean;
	}
	// Sent from actor
	| {
		type: "send-events";
		events: protocol.EventWrapper[];
	}
	| {
		type: "kv-request";
		actorId: string;
		data: protocol.KvRequestData;
		resolve: (data: protocol.KvResponseData) => void;
		reject: (error: Error) => void;
	}
	| { type: "buffer-tunnel-msg", msg: protocol.ToRivetTunnelMessage }
	| { type: "shutdown" }
	| { type: "stop" };

export async function startEnvoy(config: EnvoyConfig): Promise<EnvoyHandle> {
	const handle = startEnvoySync(config);

	// Wait for envoy start
	await handle.started();

	return handle;
}

// Must manually wait for envoy to start.
export function startEnvoySync(config: EnvoyConfig): EnvoyHandle {
	const [envoyTx, envoyRx] = unboundedChannel<ToEnvoyMessage>();
	const [startTx, startRx] = watch<void>(void 0);
	const actors: Map<string, Map<number, ActorEntry>> = new Map();

	const shared: SharedContext = {
		config,
		envoyKey: uuidv4(),
		envoyTx,
		// Start undefined
		handle: null as any,
	};

	const connHandle = startConnection(shared);

	const ctx: EnvoyContext = {
		shared,
		serverless: false,
		shuttingDown: false,
		actors,
		kvRequests: new Map(),
		nextKvRequestId: 0,
		requestToActor: new BufferMap(),
		bufferedMessages: [],
	};

	// Set shared handle
	const handle = createHandle(ctx, startRx);
	shared.handle = handle;

	log(ctx.shared)?.info({ msg: "starting envoy" });

	spawn(async () => {
		const ackInterval = setInterval(() => {
			sendCommandAck(ctx);
		}, ACK_COMMANDS_INTERVAL_MS);

		const kvCleanupInterval = setInterval(() => {
			cleanupOldKvRequests(ctx);
		}, KV_CLEANUP_INTERVAL_MS);

		let lostTimeout: NodeJS.Timeout | undefined = undefined;

		for await (const msg of envoyRx) {
			if (msg.type === "conn-message") {
				await handleConnMessage(ctx, startTx, lostTimeout, msg.message);
			} else if (msg.type === "conn-close") {
				handleConnClose(ctx, lostTimeout);
				if (msg.evict) break;
			} else if (msg.type === "send-events") {
				const stop = handleSendEvents(ctx, msg.events);

				if (stop) {
					log(ctx.shared)?.info({
						msg: "serverless actor stopped, stopping envoy"
					});
					break;
				}
			} else if (msg.type === "kv-request") {
				handleKvRequest(ctx, msg);
			} else if (msg.type === "buffer-tunnel-msg") {
				ctx.bufferedMessages.push(msg.msg);
			} else if (msg.type === "shutdown") {
				handleShutdown(ctx);
			} else if (msg.type === "stop") {
				break;
			} else {
				unreachable(msg);
			}
		}

		log(ctx.shared)?.info({
			msg: "stopping envoy",
		});

		// Cleanup
		ctx.shared.wsTx?.send({ type: "close", code: 1000, reason: "envoy.shutdown" });
		clearInterval(ackInterval);
		clearInterval(kvCleanupInterval);

		for (const request of ctx.kvRequests.values()) {
			request.reject(new Error("envoy shutting down"));
		}
		ctx.kvRequests.clear();

		for (const [, generations] of ctx.actors) {
			for (const [, entry] of generations) {
				entry.handle.close();
			}
		}
		ctx.actors.clear();
	});

	// Queue start actor
	if (shared.config.serverlessStartPayload) {
		handle.startServerless(shared.config.serverlessStartPayload);
	}

	return handle;
}

function handleConnMessage(
	ctx: EnvoyContext,
	startTx: WatchSender<void>,
	lostTimeout: NodeJS.Timeout | undefined,
	message: ToEnvoyFromConnMessage,
) {
	if (message.tag === "ToEnvoyInit") {
		ctx.shared.protocolMetadata = message.val.metadata;
		log(ctx.shared)?.info({
			msg: "received init",
			protocolMetadata: message.val.metadata,
		});

		clearTimeout(lostTimeout);
		resendUnacknowledgedEvents(ctx);
		processUnsentKvRequests(ctx);
		resendBufferedTunnelMessages(ctx);

		startTx.send();
	} else if (message.tag === "ToEnvoyCommands") {
		handleCommands(ctx, message.val);
	} else if (message.tag === "ToEnvoyAckEvents") {
		handleAckEvents(ctx, message.val);
	} else if (message.tag === "ToEnvoyKvResponse") {
		handleKvResponse(ctx, message.val);
	} else if (message.tag === "ToEnvoyTunnelMessage") {
		handleTunnelMessage(ctx, message.val);
	} else {
		unreachable(message);
	}
}

function handleConnClose(ctx: EnvoyContext, lostTimeout: NodeJS.Timeout | undefined) {
	if (!lostTimeout) {
		let lostThreshold = ctx.shared.protocolMetadata ? Number(ctx.shared.protocolMetadata.envoyLostThreshold) : 10000;
		log(ctx.shared)?.debug({
			msg: "starting runner lost timeout",
			seconds: lostThreshold / 1000,
		});

		lostTimeout = setTimeout(
			() => {
				// Remove all remaining kv requests
				for (const [_, request] of ctx.kvRequests.entries()) {
					request.reject(new EnvoyShutdownError());
				}

				ctx.kvRequests.clear();

				if (ctx.actors.size == 0) return;

				log(ctx.shared)?.warn({
					msg: "stopping all actors due to runner lost threshold",
				});

				// Stop all actors
				for (const [_, gens] of ctx.actors) {
					for (const [_, entry] of gens) {
						if (!entry.handle.isClosed()) {
							entry.handle.send({ type: "lost" });
						}
					}
				}

				ctx.actors.clear();
			},
			lostThreshold,
		);
	}
}

function handleShutdown(ctx: EnvoyContext) {
	if (ctx.shuttingDown) return;
	ctx.shuttingDown = true;

	wsSend(ctx.shared, {
		tag: "ToRivetStopping",
		val: null,
	});

	// Start shutdown checker
	spawn(async () => {
		let i = 0;

		while (true) {
			let total = 0;

			// Check for actors with open handles
			for (const gens of ctx.actors.values()) {
				const last = Array.from(gens.values())[gens.size - 1];

				if (last && !last.handle.isClosed()) total++;
			}

			// Wait until no actors remain
			if (total === 0) {
				ctx.shared.envoyTx.send({ type: "stop" });
				break;
			}

			await sleep(1000);

			if (i % 10 === 0) {
				log(ctx.shared)?.info({
					msg: "waiting on actors to stop before shutdown",
					actors: total,
				});
			}
			i++;
		}
	});
}

// MARK: Util

export function log(ctx: SharedContext) {
	if (ctx.logCached) return ctx.logCached;

	const baseLogger = ctx.config.logger ?? logger();
	if (!baseLogger) return undefined;

	ctx.logCached = baseLogger.child({
		envoyKey: ctx.envoyKey,
	});
	return ctx.logCached;
}

export function getActorEntry(
	ctx: EnvoyContext,
	actorId: string,
	generation: number,
): ActorEntry | undefined {
	return ctx.actors.get(actorId)?.get(generation);
}

// MARK: Handle

function createHandle(
	ctx: EnvoyContext,
	startRx: WatchReceiver<void>,
): EnvoyHandle {
	let startedPromise = startRx.changed();

	return {
		shutdown(immediate: boolean) {
			ctx.shared.envoyTx.send({ type: "shutdown" });
			ctx.shared.config.onShutdown();
		},

		getProtocolMetadata(): protocol.ProtocolMetadata | undefined {
			return ctx.shared.protocolMetadata;
		},

		getEnvoyKey(): string {
			return ctx.shared.envoyKey;
		},

		started(): Promise<void> {
			return startedPromise;
		},

		getActor(actorId: string, generation?: number): ActorEntry | undefined {
			return getActor(ctx, actorId, generation);
		},

		sleepActor(actorId: string, generation?: number): void {
			sendActorIntent(
				ctx,
				actorId,
				{ tag: "ActorIntentSleep", val: null },
				generation,
			);
		},

		stopActor(actorId: string, generation?: number, error?: string): void {
			sendActorIntent(
				ctx,
				actorId,
				{ tag: "ActorIntentStop", val: null },
				generation,
				error,
			);
		},

		destroyActor(actorId: string, generation?: number): void {
			sendActorIntent(
				ctx,
				actorId,
				{ tag: "ActorIntentStop", val: null },
				generation,
			);
		},

		setAlarm(
			actorId: string,
			alarmTs: number | null,
			generation?: number,
		): void {
			const entry = getActor(ctx, actorId, generation);
			if (!entry) return;
			entry.handle.send({
				type: "set-alarm",
				alarmTs: alarmTs !== null ? BigInt(alarmTs) : null,
			});
		},

		async kvGet(
			actorId: string,
			keys: Uint8Array[],
		): Promise<(Uint8Array | null)[]> {
			const kvKeys = keys.map(toBuffer);
			const response = await sendKvRequest(ctx, actorId, {
				tag: "KvGetRequest",
				val: { keys: kvKeys },
			});

			const val = (
				response as {
					tag: "KvGetResponse";
					val: protocol.KvGetResponse;
				}
			).val;
			const responseKeys = val.keys.map(
				(k: ArrayBuffer) => new Uint8Array(k),
			);
			const responseValues = val.values.map(
				(v: ArrayBuffer) => new Uint8Array(v),
			);

			const result: (Uint8Array | null)[] = [];
			for (const requestedKey of keys) {
				let found = false;
				for (let i = 0; i < responseKeys.length; i++) {
					if (uint8ArraysEqual(requestedKey, responseKeys[i])) {
						result.push(responseValues[i]);
						found = true;
						break;
					}
				}
				if (!found) {
					result.push(null);
				}
			}
			return result;
		},

		async kvListAll(
			actorId: string,
			options?: KvListOptions,
		): Promise<[Uint8Array, Uint8Array][]> {
			const response = await sendKvRequest(ctx, actorId, {
				tag: "KvListRequest",
				val: {
					query: { tag: "KvListAllQuery", val: null },
					reverse: options?.reverse ?? null,
					limit:
						options?.limit !== undefined
							? BigInt(options.limit)
							: null,
				},
			});
			return parseListResponse(response);
		},

		async kvListRange(
			actorId: string,
			start: Uint8Array,
			end: Uint8Array,
			exclusive?: boolean,
			options?: KvListOptions,
		): Promise<[Uint8Array, Uint8Array][]> {
			const response = await sendKvRequest(ctx, actorId, {
				tag: "KvListRequest",
				val: {
					query: {
						tag: "KvListRangeQuery",
						val: {
							start: toBuffer(start),
							end: toBuffer(end),
							exclusive: exclusive ?? false,
						},
					},
					reverse: options?.reverse ?? null,
					limit:
						options?.limit !== undefined
							? BigInt(options.limit)
							: null,
				},
			});
			return parseListResponse(response);
		},

		async kvListPrefix(
			actorId: string,
			prefix: Uint8Array,
			options?: KvListOptions,
		): Promise<[Uint8Array, Uint8Array][]> {
			const response = await sendKvRequest(ctx, actorId, {
				tag: "KvListRequest",
				val: {
					query: {
						tag: "KvListPrefixQuery",
						val: { key: toBuffer(prefix) },
					},
					reverse: options?.reverse ?? null,
					limit:
						options?.limit !== undefined
							? BigInt(options.limit)
							: null,
				},
			});
			return parseListResponse(response);
		},

		async kvPut(
			actorId: string,
			entries: [Uint8Array, Uint8Array][],
		): Promise<void> {
			const keys = entries.map(([k]) => toBuffer(k));
			const values = entries.map(([, v]) => toBuffer(v));
			await sendKvRequest(ctx, actorId, {
				tag: "KvPutRequest",
				val: { keys, values },
			});
		},

		async kvDelete(
			actorId: string,
			keys: Uint8Array[],
		): Promise<void> {
			await sendKvRequest(ctx, actorId, {
				tag: "KvDeleteRequest",
				val: { keys: keys.map(toBuffer) },
			});
		},

		async kvDeleteRange(
			actorId: string,
			start: Uint8Array,
			end: Uint8Array,
		): Promise<void> {
			await sendKvRequest(ctx, actorId, {
				tag: "KvDeleteRangeRequest",
				val: { start: toBuffer(start), end: toBuffer(end) },
			});
		},

		async kvDrop(actorId: string): Promise<void> {
			await sendKvRequest(ctx, actorId, {
				tag: "KvDropRequest",
				val: null,
			});
		},

		restoreHibernatingRequests(
			actorId: string,
			metaEntries: HibernatingWebSocketMetadata[],
		) {
			const actor = getActor(ctx, actorId);
			if (!actor) {
				throw new Error(
					`Actor ${actorId} not found for restoring hibernating requests`,
				);
			}

			actor.handle.send({ type: "hws-restore", metaEntries });
		},

		sendHibernatableWebSocketMessageAck(
			gatewayId: protocol.GatewayId,
			requestId: protocol.RequestId,
			clientMessageIndex: number,
		) {
			sendHibernatableWebSocketMessageAck(ctx, gatewayId, requestId, clientMessageIndex);
		},

		startServerless(payload: ArrayBuffer) {
			if (ctx.serverless) throw new Error("Already started serverless actor");
			ctx.serverless = true;

			let version = new DataView(payload).getUint16(0, true);

			if (version != protocol.VERSION)
				throw new Error(`Serverless start payload does not match protocol version: ${version} vs ${protocol.VERSION}`);

			// Skip first 2 bytes (version)
			const message = protocol.decodeToEnvoy(new Uint8Array(payload, 2));

			if (message.tag !== "ToEnvoyCommands") throw new Error("invalid serverless body");
			if (message.val.length !== 1) throw new Error("invalid serverless body");
			if (message.val[0].inner.tag !== "CommandStartActor") throw new Error("invalid serverless body");

			// Wait for envoy to start before adding message
			startedPromise.then(() => {
				log(ctx.shared)?.debug({
					msg: "received serverless start",
					data: stringifyToEnvoy(message),
				});
				ctx.shared.envoyTx.send({ type: "conn-message", message });
			});
		}
	};
}

function sendActorIntent(
	ctx: EnvoyContext,
	actorId: string,
	intent: protocol.ActorIntent,
	generation?: number,
	error?: string,
): void {
	const entry = getActor(ctx, actorId, generation);
	if (!entry) return;
	entry.handle.send({
		type: "intent",
		intent,
		error,
	});
}

function sendKvRequest(
	ctx: EnvoyContext,
	actorId: string,
	data: protocol.KvRequestData,
): Promise<protocol.KvResponseData> {
	return new Promise((resolve, reject) => {
		ctx.shared.envoyTx.send({
			type: "kv-request",
			actorId,
			data,
			resolve,
			reject,
		});
	});
}

function toBuffer(arr: Uint8Array): ArrayBuffer {
	return arr.buffer.slice(
		arr.byteOffset,
		arr.byteOffset + arr.byteLength,
	) as ArrayBuffer;
}

function parseListResponse(
	response: protocol.KvResponseData,
): [Uint8Array, Uint8Array][] {
	const val = (
		response as {
			tag: "KvListResponse";
			val: protocol.KvListResponse;
		}
	).val;
	const result: [Uint8Array, Uint8Array][] = [];
	for (let i = 0; i < val.keys.length; i++) {
		const key = val.keys[i];
		const value = val.values[i];
		if (key && value) {
			result.push([new Uint8Array(key), new Uint8Array(value)]);
		}
	}
	return result;
}

export function getActor(
	ctx: EnvoyContext,
	actorId: string,
	generation?: number,
): ActorEntry | undefined {
	const gens = ctx.actors.get(actorId);
	if (!gens || gens.size === 0) return undefined;

	if (generation !== undefined) {
		return gens.get(generation);
	}

	// Return highest generation non-closed (active) entry
	for (const entry of Array.from(gens.values()).reverse()) {
		if (!entry.handle.isClosed()) {
			return entry;
		}
	}
	return undefined;
}

function uint8ArraysEqual(a: Uint8Array, b: Uint8Array): boolean {
	if (a.length !== b.length) return false;
	for (let i = 0; i < a.length; i++) {
		if (a[i] !== b[i]) return false;
	}
	return true;
}
