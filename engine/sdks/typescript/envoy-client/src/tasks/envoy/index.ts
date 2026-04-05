import * as protocol from "@rivetkit/engine-envoy-protocol";
import type { UnboundedSender } from "antiox/sync/mpsc";
import { unboundedChannel } from "antiox/sync/mpsc";
import { v4 as uuidv4 } from "uuid";
import type { ToActor } from "../actor.js";
import type { EnvoyConfig } from "../../config.js";
import type { EnvoyHandle, KvListOptions } from "../../handle.js";
import { startConnection } from "../connection.js";
import type { SharedContext } from "../../context.js";
import { logger } from "../../log.js";
import { stringifyToRivet } from "../../stringify.js";
import { unreachable } from "antiox/panic";
import {
	ACK_COMMANDS_INTERVAL_MS,
	handleCommands,
	sendCommandAck,
} from "./commands.js";
import {
	handleAckEvents,
	handleCommandStopActorComplete,
	handleSendEvents,
	resendUnacknowledgedEvents,
} from "./events.js";
import {
	KV_CLEANUP_INTERVAL_MS,
	type KvRequestEntry,
	cleanupOldKvRequests,
	handleKvRequest,
	handleKvResponse,
	processUnsentKvRequests,
} from "./kv.js";

export interface EnvoyContext {
	shared: SharedContext;
	protocolMetadata?: protocol.ProtocolMetadata;
	actors: Map<string, Map<number, ActorEntry>>;
	kvRequests: Map<number, KvRequestEntry>;
	nextKvRequestId: number;
}

export interface ActorEntry {
	handle: UnboundedSender<ToActor>;
	eventHistory: protocol.EventWrapper[];
	lastCommandIdx: bigint;
}

/**
 * Message coming from the connection.
 *
 * Ping is handled by the connection task.
 */
export type ToEnvoyConnMessage = Exclude<
	protocol.ToEnvoy,
	{ tag: "ToEnvoyPing" }
>;

export type ToEnvoyMessage =
	// Inbound from connection
	| { type: "conn-message"; message: ToEnvoyConnMessage }
	// Sent from actor
	| {
			type: "send-events";
			events: protocol.EventWrapper[];
	  }
	| {
			type: "command-stop-actor-complete";
			actorId: string;
			generation: number;
			checkpointIndex: bigint;
			code: protocol.StopCode;
			message: string | null;
	  }
	| {
			type: "kv-request";
			actorId: string;
			data: protocol.KvRequestData;
			resolve: (data: protocol.KvResponseData) => void;
			reject: (error: Error) => void;
	  };

export async function startEnvoy(config: EnvoyConfig) {
	const [envoyTx, envoyRx] = unboundedChannel<ToEnvoyMessage>();
	const actors: Map<string, Map<number, ActorEntry>> = new Map();

	const handle = createHandle(actors, envoyTx);
	const shared: SharedContext = {
		config,
		envoyKey: uuidv4(),
		envoyTx,
		handle,
	};

	startConnection(shared);

	const ctx: EnvoyContext = {
		shared,
		actors,
		kvRequests: new Map(),
		nextKvRequestId: 0,
	};

	log(ctx.shared)?.info({ msg: "starting envoy" });

	const ackInterval = setInterval(() => {
		sendCommandAck(ctx);
	}, ACK_COMMANDS_INTERVAL_MS);

	const kvCleanupInterval = setInterval(() => {
		cleanupOldKvRequests(ctx);
	}, KV_CLEANUP_INTERVAL_MS);

	for await (const msg of envoyRx) {
		if (msg.type === "conn-message") {
			await handleConnMessage(ctx, msg.message);
		} else if (msg.type === "send-events") {
			handleSendEvents(ctx, msg.events);
		} else if (msg.type === "command-stop-actor-complete") {
			handleCommandStopActorComplete(ctx, msg);
		} else if (msg.type === "kv-request") {
			handleKvRequest(ctx, msg);
		} else {
			unreachable(msg);
		}
	}

	// Cleanup
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
}

async function handleConnMessage(
	ctx: EnvoyContext,
	message: ToEnvoyConnMessage,
) {
	if (message.tag === "ToEnvoyInit") {
		ctx.protocolMetadata = message.val.metadata;
		log(ctx.shared)?.info({
			msg: "received init",
			protocolMetadata: message.val.metadata,
		});

		resendUnacknowledgedEvents(ctx);
		processUnsentKvRequests(ctx);
	} else if (message.tag === "ToEnvoyCommands") {
		await handleCommands(ctx, message.val);
	} else if (message.tag === "ToEnvoyAckEvents") {
		handleAckEvents(ctx, message.val);
	} else if (message.tag === "ToEnvoyKvResponse") {
		handleKvResponse(ctx, message.val);
	} else if (message.tag === "ToEnvoyTunnelMessage") {
		// TODO:
	} else {
		unreachable(message);
	}
}

// MARK: Util

export function wsSend(ctx: EnvoyContext, message: protocol.ToRivet) {
	log(ctx.shared)?.debug({
		msg: "sending message",
		data: stringifyToRivet(message),
	});

	if (!ctx.shared.wsTx) {
		log(ctx.shared)?.warn({
			msg: "websocket not available for sending, events will be resent on reconnect",
		});
		return;
	}

	const encoded = protocol.encodeToRivet(message);
	ctx.shared.wsTx.send({ type: "send", data: encoded });
}

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
	actors: Map<string, Map<number, ActorEntry>>,
	envoyTx: UnboundedSender<ToEnvoyMessage>,
): EnvoyHandle {
	function findActor(
		actorId: string,
		generation?: number,
	): ActorEntry | undefined {
		const gens = actors.get(actorId);
		if (!gens || gens.size === 0) return undefined;

		if (generation !== undefined) {
			return gens.get(generation);
		}

		// Return first non-closed (active) entry
		for (const entry of gens.values()) {
			if (!entry.handle.isClosed()) {
				return entry;
			}
		}
		return undefined;
	}

	function sendActorIntent(
		actorId: string,
		intent: protocol.ActorIntent,
		generation?: number,
	): void {
		const entry = findActor(actorId, generation);
		if (!entry) return;
		entry.handle.send({
			type: "actor-intent",
			commandIdx: 0n,
			intent,
		});
	}

	function sendKvRequest(
		actorId: string,
		data: protocol.KvRequestData,
	): Promise<protocol.KvResponseData> {
		return new Promise((resolve, reject) => {
			envoyTx.send({
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

	return {
		sleepActor(actorId: string, generation?: number): void {
			sendActorIntent(
				actorId,
				{ tag: "ActorIntentSleep", val: null },
				generation,
			);
		},

		stopActor(actorId: string, generation?: number): void {
			sendActorIntent(
				actorId,
				{ tag: "ActorIntentStop", val: null },
				generation,
			);
		},

		destroyActor(actorId: string, generation?: number): void {
			sendActorIntent(
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
			const entry = findActor(actorId, generation);
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
			const response = await sendKvRequest(actorId, {
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
			const response = await sendKvRequest(actorId, {
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
			const response = await sendKvRequest(actorId, {
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
			const response = await sendKvRequest(actorId, {
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
			await sendKvRequest(actorId, {
				tag: "KvPutRequest",
				val: { keys, values },
			});
		},

		async kvDelete(
			actorId: string,
			keys: Uint8Array[],
		): Promise<void> {
			await sendKvRequest(actorId, {
				tag: "KvDeleteRequest",
				val: { keys: keys.map(toBuffer) },
			});
		},

		async kvDeleteRange(
			actorId: string,
			start: Uint8Array,
			end: Uint8Array,
		): Promise<void> {
			await sendKvRequest(actorId, {
				tag: "KvDeleteRangeRequest",
				val: { start: toBuffer(start), end: toBuffer(end) },
			});
		},

		async kvDrop(actorId: string): Promise<void> {
			await sendKvRequest(actorId, {
				tag: "KvDropRequest",
				val: null,
			});
		},
	};
}

function uint8ArraysEqual(a: Uint8Array, b: Uint8Array): boolean {
	if (a.length !== b.length) return false;
	for (let i = 0; i < a.length; i++) {
		if (a[i] !== b[i]) return false;
	}
	return true;
}
