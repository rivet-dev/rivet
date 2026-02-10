import * as cbor from "cbor-x";
import { isCborSerializable } from "@/common/utils";
import {
	CURRENT_VERSION as ACTOR_PERSIST_CURRENT_VERSION,
	QUEUE_MESSAGE_VERSIONED,
	QUEUE_METADATA_VERSIONED,
} from "@/schemas/actor-persist/versioned";
import { promiseWithResolvers } from "@/utils";
import type { AnyDatabaseProvider } from "../database";
import type { ActorDriver } from "../driver";
import * as errors from "../errors";
import { decodeQueueMessageKey, KEYS, makeQueueMessageKey } from "./keys";
import type { ActorInstance } from "./mod";

export interface QueueMessage {
	id: bigint;
	name: string;
	body: unknown;
	createdAt: number;
	failureCount: number;
	availableAt: number;
	inFlight: boolean;
	inFlightAt?: number;
}

export interface QueueCompletionResult {
	status: "completed" | "timedOut";
	response?: unknown;
}

interface QueueMetadata {
	nextId: bigint;
	size: number;
}

interface EnqueueOptions {
	deferWaiters?: boolean;
}

interface QueueWaiter {
	id: string;
	nameSet: Set<string>;
	count: number;
	wait: boolean;
	resolve: (messages: QueueMessage[]) => void;
	reject: (error: Error) => void;
	signal?: AbortSignal;
	timeoutHandle?: ReturnType<typeof setTimeout>;
}

interface QueueNameWaiter {
	id: string;
	nameSet: Set<string>;
	resolve: () => void;
	reject: (error: Error) => void;
	signal?: AbortSignal;
	abortHandler?: () => void;
}

interface QueueCompletionWaiter {
	id: string;
	messageId: bigint;
	resolve: (result: QueueCompletionResult) => void;
	reject: (error: Error) => void;
	timeoutHandle?: ReturnType<typeof setTimeout>;
}

interface MessageListener {
	nameSet: Set<string>;
	resolve: () => void;
	reject: (error: Error) => void;
	actorAbortCleanup?: () => void;
	signal?: AbortSignal;
	signalAbortCleanup?: () => void;
}

const DEFAULT_METADATA: QueueMetadata = {
	nextId: 1n,
	size: 0,
};

const PENDING_WARNING_MS = 30_000;
const BACKOFF_INITIAL_MS = 1_000;
const BACKOFF_MAX_MS = 5 * 60_000;

export class QueueManager<S, CP, CS, V, I, DB extends AnyDatabaseProvider> {
	#actor: ActorInstance<S, CP, CS, V, I, DB>;
	#driver: ActorDriver;
	#waiters = new Map<string, QueueWaiter>();
	#nameWaiters = new Map<string, QueueNameWaiter>();
	#completionWaiters = new Map<bigint, QueueCompletionWaiter>();
	#metadata: QueueMetadata = { ...DEFAULT_METADATA };
	#pendingMessageId: bigint | undefined;
	#pendingWarningHandle: ReturnType<typeof setTimeout> | undefined;
	#redeliveryTimeout: ReturnType<typeof setTimeout> | undefined;
	#redeliveryAt: number | undefined;
	#messageListeners = new Set<MessageListener>();

	constructor(
		actor: ActorInstance<S, CP, CS, V, I, DB>,
		driver: ActorDriver,
	) {
		this.#actor = actor;
		this.#driver = driver;
	}

	/** Returns the current number of messages in the queue. */
	get size(): number {
		return this.#metadata.size;
	}

	/** Loads queue metadata from storage and initializes internal state. */
	async initialize(): Promise<void> {
		const [metadataBuffer] = await this.#driver.kvBatchGet(this.#actor.id, [
			KEYS.QUEUE_METADATA,
		]);
		if (!metadataBuffer) {
			await this.#driver.kvBatchPut(this.#actor.id, [
				[KEYS.QUEUE_METADATA, this.#serializeMetadata()],
			]);
			this.#actor.inspector.updateQueueSize(this.#metadata.size);
			return;
		}
		try {
			const decoded =
				QUEUE_METADATA_VERSIONED.deserializeWithEmbeddedVersion(
					metadataBuffer,
				);
			this.#metadata.nextId = decoded.nextId;
			this.#metadata.size = Number(decoded.size);
		} catch (error) {
			this.#actor.rLog.error({
				msg: "failed to decode queue metadata, rebuilding from messages",
				error,
			});
			await this.#rebuildMetadata();
		}
		this.#actor.inspector.updateQueueSize(this.#metadata.size);

		await this.#recoverInFlightMessages();
	}

	/** Adds a message to the queue with the given name and body. */
	async enqueue(
		name: string,
		body: unknown,
		options: EnqueueOptions = {},
	): Promise<QueueMessage> {
		this.#actor.assertReady();

		const sizeLimit = this.#actor.config.options.maxQueueSize;
		if (this.#metadata.size >= sizeLimit) {
			throw new errors.QueueFull(sizeLimit);
		}

		let invalidPath = "";
		if (
			!isCborSerializable(body, (path) => {
				invalidPath = path;
			})
		) {
			throw new errors.QueueMessageInvalid(invalidPath);
		}

		const createdAt = Date.now();
		const bodyCborBuffer = cbor.encode(body);
		const availableAt = createdAt;
		const encodedMessage =
			QUEUE_MESSAGE_VERSIONED.serializeWithEmbeddedVersion(
				{
					name,
					body: new Uint8Array(bodyCborBuffer).buffer as ArrayBuffer,
					createdAt: BigInt(createdAt),
					failureCount: 0,
					availableAt: BigInt(availableAt),
					inFlight: false,
					inFlightAt: null,
				},
				ACTOR_PERSIST_CURRENT_VERSION,
			);
		const encodedSize = encodedMessage.byteLength;
		if (encodedSize > this.#actor.config.options.maxQueueMessageSize) {
			throw new errors.QueueMessageTooLarge(
				encodedSize,
				this.#actor.config.options.maxQueueMessageSize,
			);
		}

		const id = this.#metadata.nextId;
		const messageKey = makeQueueMessageKey(id);

		// Update metadata before writing so we can batch both writes
		this.#metadata.nextId = id + 1n;
		this.#metadata.size += 1;
		const encodedMetadata = this.#serializeMetadata();

		// Batch write message and metadata together
		await this.#driver.kvBatchPut(this.#actor.id, [
			[messageKey, encodedMessage],
			[KEYS.QUEUE_METADATA, encodedMetadata],
		]);

		this.#actor.inspector.updateQueueSize(this.#metadata.size);

		const message: QueueMessage = {
			id,
			name,
			body,
			createdAt,
			failureCount: 0,
			availableAt,
			inFlight: false,
			inFlightAt: undefined,
		};

		this.#actor.resetSleepTimer();
		if (!options.deferWaiters) {
			await this.#maybeResolveWaiters();
		}
		this.#notifyMessageListeners(name);

		return message;
	}

	async enqueueAndWait(
		name: string,
		body: unknown,
		timeout?: number,
	): Promise<QueueCompletionResult> {
		const message = await this.enqueue(name, body, {
			deferWaiters: true,
		});
		const completionPromise = this.waitForCompletion(message.id, timeout);
		await this.#maybeResolveWaiters();
		return await completionPromise;
	}

	/** Receives messages from the queue matching the given names. Waits until messages are available or timeout is reached. */
	async receive(
		names: string[],
		count: number,
		timeout?: number,
		abortSignal?: AbortSignal,
		wait: boolean = false,
	): Promise<QueueMessage[] | undefined> {
		this.#actor.assertReady();
		if (this.#pendingMessageId !== undefined) {
			throw new errors.QueueMessagePending();
		}
		const limitedCount = Math.max(1, count);
		const nameSet = new Set(names);

		const immediate = await this.#drainMessages(
			nameSet,
			limitedCount,
			wait,
		);
		if (immediate.length > 0 || timeout === 0) {
			return timeout === 0 && immediate.length === 0 ? [] : immediate;
		}

		const { promise, resolve, reject } =
			promiseWithResolvers<QueueMessage[]>();
		const waiterId = crypto.randomUUID();
		const waiter: QueueWaiter = {
			id: waiterId,
			nameSet,
			count: limitedCount,
			wait,
			resolve,
			reject,
			signal: abortSignal,
		};

		if (timeout !== undefined) {
			waiter.timeoutHandle = setTimeout(() => {
				this.#waiters.delete(waiterId);
				resolve([]);
			}, timeout);
		}

		const onAbort = () => {
			this.#waiters.delete(waiterId);
			if (waiter.timeoutHandle) {
				clearTimeout(waiter.timeoutHandle);
			}
			reject(new errors.ActorAborted());
		};
		const onStop = () => {
			this.#waiters.delete(waiterId);
			if (waiter.timeoutHandle) {
				clearTimeout(waiter.timeoutHandle);
			}
			reject(new errors.ActorAborted());
		};
		const actorAbortSignal = this.#actor.abortSignal;
		if (actorAbortSignal.aborted) {
			onStop();
			return promise;
		}
		actorAbortSignal.addEventListener("abort", onStop, { once: true });

		if (abortSignal) {
			if (abortSignal.aborted) {
				onAbort();
				return promise;
			}
			abortSignal.addEventListener("abort", onAbort, { once: true });
		}

		this.#waiters.set(waiterId, waiter);
		return promise;
	}

	/** Waits for a specific queue message to complete. */
	async waitForCompletion(
		messageId: bigint,
		timeout?: number,
	): Promise<QueueCompletionResult> {
		const { promise, resolve, reject } =
			promiseWithResolvers<QueueCompletionResult>();
		const waiterId = crypto.randomUUID();

		const waiter: QueueCompletionWaiter = {
			id: waiterId,
			messageId,
			resolve,
			reject,
		};

		if (timeout !== undefined) {
			waiter.timeoutHandle = setTimeout(() => {
				this.#completionWaiters.delete(messageId);
				resolve({ status: "timedOut" });
			}, timeout);
		}

		this.#completionWaiters.set(messageId, waiter);
		return promise;
	}

	/** Completes a pending message and optionally responds to any waiter. */
	async complete(message: QueueMessage, response?: unknown): Promise<void> {
		if (this.#pendingMessageId !== message.id) {
			throw new errors.QueueAlreadyCompleted();
		}
		this.#pendingMessageId = undefined;
		if (this.#pendingWarningHandle) {
			clearTimeout(this.#pendingWarningHandle);
			this.#pendingWarningHandle = undefined;
		}

		await this.#removeMessages([message], { resolveWaiters: false });
		this.#resolveCompletionWaiter(message.id, {
			status: "completed",
			response,
		});

		await this.#maybeResolveWaiters();
	}

	/** Waits for messages with any of the specified names to appear in the queue. */
	async waitForNames(
		names: string[],
		abortSignal?: AbortSignal,
	): Promise<void> {
		const nameSet = new Set(names);
		const existing = await this.#loadQueueMessages();
		if (existing.some((message) => nameSet.has(message.name))) {
			return;
		}

		return await new Promise<void>((resolve, reject) => {
			const listener: MessageListener = {
				nameSet,
				resolve: () => {
					this.#removeMessageListener(listener);
					resolve();
				},
				reject: (error) => {
					this.#removeMessageListener(listener);
					reject(error);
				},
			};

			const actorAbortSignal = this.#actor.abortSignal;
			const onActorAbort = () =>
				listener.reject(new errors.ActorAborted());
			if (actorAbortSignal.aborted) {
				onActorAbort();
				return;
			}
			actorAbortSignal.addEventListener("abort", onActorAbort, {
				once: true,
			});
			listener.actorAbortCleanup = () =>
				actorAbortSignal.removeEventListener("abort", onActorAbort);

			if (abortSignal) {
				const onAbort = () =>
					listener.reject(new errors.ActorAborted());
				if (abortSignal.aborted) {
					onAbort();
					return;
				}
				abortSignal.addEventListener("abort", onAbort, { once: true });
				listener.signalAbortCleanup = () =>
					abortSignal.removeEventListener("abort", onAbort);
			}

			this.#messageListeners.add(listener);
		});
	}

	/** Returns all messages currently in the queue without removing them. */
	async getMessages(): Promise<QueueMessage[]> {
		return await this.#loadQueueMessages();
	}

	/** Deletes messages matching the provided IDs. Returns the IDs that were removed. */
	async deleteMessagesById(ids: bigint[]): Promise<bigint[]> {
		if (ids.length === 0) {
			return [];
		}
		const idSet = new Set(ids.map((id) => id.toString()));
		const entries = await this.#loadQueueMessages();
		const toRemove = entries.filter((entry) =>
			idSet.has(entry.id.toString()),
		);
		if (toRemove.length === 0) {
			return [];
		}
		await this.#removeMessages(toRemove, { resolveWaiters: true });
		return toRemove.map((entry) => entry.id);
	}

	async #drainMessages(
		nameSet: Set<string>,
		count: number,
		wait: boolean,
	): Promise<QueueMessage[]> {
		if (this.#metadata.size === 0) {
			return [];
		}
		const now = Date.now();
		const entries = await this.#loadQueueMessages();
		const matched = entries.filter(
			(entry) => nameSet.has(entry.name) && !entry.inFlight,
		);
		if (matched.length === 0) {
			return [];
		}

		const eligible = matched.filter((entry) => entry.availableAt <= now);
		if (eligible.length === 0) {
			this.#scheduleRedelivery(matched);
			return [];
		}

		const selected = eligible.slice(0, wait ? 1 : count);
		if (wait) {
			await this.#markMessageInFlight(selected[0], now);
			return [selected[0]];
		}

		await this.#removeMessages(selected, { resolveWaiters: true });

		// Emit trace events for received messages
		for (const message of selected) {
			this.#actor.emitTraceEvent("queue.message.receive", {
				"rivet.queue.name": message.name,
				"rivet.queue.message_id": message.id.toString(),
				"rivet.queue.created_at_ms": message.createdAt,
				"rivet.queue.latency_ms": now - message.createdAt,
			});
		}

		return selected;
	}

	async #loadQueueMessages(): Promise<QueueMessage[]> {
		const entries = await this.#driver.kvListPrefix(
			this.#actor.id,
			KEYS.QUEUE_PREFIX,
		);
		const decoded: QueueMessage[] = [];
		for (const [key, value] of entries) {
			try {
				const messageId = decodeQueueMessageKey(key);
				const decodedPayload =
					QUEUE_MESSAGE_VERSIONED.deserializeWithEmbeddedVersion(
						value,
					);
				const body = cbor.decode(new Uint8Array(decodedPayload.body));
				const failureCount =
					decodedPayload.failureCount !== undefined &&
					decodedPayload.failureCount !== null
						? Number(decodedPayload.failureCount)
						: 0;
				const availableAt =
					decodedPayload.availableAt !== undefined &&
					decodedPayload.availableAt !== null
						? Number(decodedPayload.availableAt)
						: Number(decodedPayload.createdAt);
				const inFlight = decodedPayload.inFlight ?? false;
				const inFlightAt =
					decodedPayload.inFlightAt !== undefined &&
					decodedPayload.inFlightAt !== null
						? Number(decodedPayload.inFlightAt)
						: undefined;
				decoded.push({
					id: messageId,
					name: decodedPayload.name,
					body,
					createdAt: Number(decodedPayload.createdAt),
					failureCount,
					availableAt,
					inFlight,
					inFlightAt,
				});
			} catch (error) {
				this.#actor.rLog.error({
					msg: "failed to decode queue message",
					error,
				});
			}
		}
		decoded.sort((a, b) => (a.id < b.id ? -1 : a.id > b.id ? 1 : 0));
		if (this.#metadata.size !== decoded.length) {
			this.#metadata.size = decoded.length;
			this.#actor.inspector.updateQueueSize(this.#metadata.size);
		}
		return decoded;
	}

	#removeMessageListener(listener: MessageListener): void {
		if (this.#messageListeners.delete(listener)) {
			listener.actorAbortCleanup?.();
			listener.signalAbortCleanup?.();
		}
	}

	#notifyMessageListeners(name: string): void {
		if (this.#messageListeners.size === 0) {
			return;
		}
		for (const listener of [...this.#messageListeners]) {
			if (!listener.nameSet.has(name)) {
				continue;
			}
			this.#removeMessageListener(listener);
			listener.resolve();
		}
	}

	async #removeMessages(
		messages: QueueMessage[],
		options: { resolveWaiters: boolean },
	): Promise<void> {
		if (messages.length === 0) {
			return;
		}
		const keys = messages.map((message) => makeQueueMessageKey(message.id));

		// Update metadata
		this.#metadata.size = Math.max(
			0,
			this.#metadata.size - messages.length,
		);

		// Delete messages and update metadata
		// Note: kvBatchDelete doesn't support mixed operations, so we do two calls
		await this.#driver.kvBatchDelete(this.#actor.id, keys);
		await this.#driver.kvBatchPut(this.#actor.id, [
			[KEYS.QUEUE_METADATA, this.#serializeMetadata()],
		]);

		this.#actor.inspector.updateQueueSize(this.#metadata.size);

		if (options.resolveWaiters) {
			for (const message of messages) {
				this.#resolveCompletionWaiter(message.id, {
					status: "completed",
					response: undefined,
				});
			}
		}
	}

	async #maybeResolveWaiters() {
		if (this.#pendingMessageId !== undefined) {
			return;
		}
		if (this.#redeliveryTimeout) {
			clearTimeout(this.#redeliveryTimeout);
			this.#redeliveryTimeout = undefined;
			this.#redeliveryAt = undefined;
		}
		const hasReceiveWaiters = this.#waiters.size > 0;
		const hasNameWaiters = this.#nameWaiters.size > 0;
		if (!hasReceiveWaiters && !hasNameWaiters) {
			return;
		}

		if (hasNameWaiters) {
			const entries = await this.#loadQueueMessages();
			const now = Date.now();
			const nameWaiters = [...this.#nameWaiters.values()];
			for (const waiter of nameWaiters) {
				if (waiter.signal?.aborted) {
					this.#nameWaiters.delete(waiter.id);
					waiter.reject(new errors.ActorAborted());
					continue;
				}

				const hasMatch = entries.some(
					(message) =>
						waiter.nameSet.has(message.name) &&
						!message.inFlight &&
						message.availableAt <= now,
				);
				if (!hasMatch) {
					continue;
				}

				this.#nameWaiters.delete(waiter.id);
				if (waiter.abortHandler) {
					waiter.signal?.removeEventListener(
						"abort",
						waiter.abortHandler,
					);
				}
				waiter.resolve();
			}
		}

		if (!hasReceiveWaiters) {
			return;
		}
		const pending = [...this.#waiters.values()];
		for (const waiter of pending) {
			if (waiter.signal?.aborted) {
				this.#waiters.delete(waiter.id);
				waiter.reject(new errors.ActorAborted());
				continue;
			}

			const messages = await this.#drainMessages(
				waiter.nameSet,
				waiter.count,
				waiter.wait,
			);
			if (messages.length === 0) {
				continue;
			}
			this.#waiters.delete(waiter.id);
			if (waiter.timeoutHandle) {
				clearTimeout(waiter.timeoutHandle);
			}
			waiter.resolve(messages);
			if (waiter.wait) {
				break;
			}
		}
	}

	/** Rebuilds metadata by scanning existing queue messages. Used when metadata is corrupted. */
	async #rebuildMetadata(): Promise<void> {
		const entries = await this.#driver.kvListPrefix(
			this.#actor.id,
			KEYS.QUEUE_PREFIX,
		);

		let maxId = 0n;
		for (const [key] of entries) {
			try {
				const messageId = decodeQueueMessageKey(key);
				if (messageId > maxId) {
					maxId = messageId;
				}
			} catch {
				// Skip malformed keys
			}
		}

		this.#metadata.nextId = maxId + 1n;
		this.#metadata.size = entries.length;

		await this.#driver.kvBatchPut(this.#actor.id, [
			[KEYS.QUEUE_METADATA, this.#serializeMetadata()],
		]);
		this.#actor.inspector.updateQueueSize(this.#metadata.size);
	}

	async #markMessageInFlight(
		message: QueueMessage,
		now: number,
	): Promise<void> {
		if (message.inFlight) {
			throw new errors.QueueMessagePending();
		}

		message.inFlight = true;
		message.inFlightAt = now;

		await this.#persistMessage(message);

		this.#pendingMessageId = message.id;
		this.#pendingWarningHandle = setTimeout(() => {
			if (this.#pendingMessageId === message.id) {
				this.#actor.rLog.warn({
					msg: "queue message pending for over 30s",
					messageId: message.id.toString(),
					name: message.name,
				});
			}
		}, PENDING_WARNING_MS);
	}

	async #persistMessage(message: QueueMessage): Promise<void> {
		const bodyCborBuffer = cbor.encode(message.body);
		const encodedMessage =
			QUEUE_MESSAGE_VERSIONED.serializeWithEmbeddedVersion(
				{
					name: message.name,
					body: new Uint8Array(bodyCborBuffer).buffer as ArrayBuffer,
					createdAt: BigInt(message.createdAt),
					failureCount: message.failureCount,
					availableAt: BigInt(message.availableAt),
					inFlight: message.inFlight,
					inFlightAt:
						message.inFlightAt !== undefined
							? BigInt(message.inFlightAt)
							: null,
				},
				ACTOR_PERSIST_CURRENT_VERSION,
			);

		await this.#driver.kvBatchPut(this.#actor.id, [
			[makeQueueMessageKey(message.id), encodedMessage],
		]);
	}

	async #recoverInFlightMessages(): Promise<void> {
		const entries = await this.#driver.kvListPrefix(
			this.#actor.id,
			KEYS.QUEUE_PREFIX,
		);

		const updates: [Uint8Array, Uint8Array][] = [];
		const now = Date.now();

		for (const [key, value] of entries) {
			try {
				const messageId = decodeQueueMessageKey(key);
				const decodedPayload =
					QUEUE_MESSAGE_VERSIONED.deserializeWithEmbeddedVersion(
						value,
					);
				const inFlight = decodedPayload.inFlight ?? false;
				if (!inFlight) {
					continue;
				}

				const failureCount =
					(decodedPayload.failureCount !== undefined &&
					decodedPayload.failureCount !== null
						? Number(decodedPayload.failureCount)
						: 0) + 1;
				const availableAt = now + this.#computeBackoffMs(failureCount);

				const updatedMessage =
					QUEUE_MESSAGE_VERSIONED.serializeWithEmbeddedVersion(
						{
							name: decodedPayload.name,
							body: decodedPayload.body,
							createdAt: decodedPayload.createdAt,
							failureCount,
							availableAt: BigInt(availableAt),
							inFlight: false,
							inFlightAt: null,
						},
						ACTOR_PERSIST_CURRENT_VERSION,
					);

				updates.push([key, updatedMessage]);

				this.#actor.rLog.warn({
					msg: "recovering in-flight queue message",
					messageId: messageId.toString(),
					failureCount,
					availableAt,
				});
			} catch (error) {
				this.#actor.rLog.error({
					msg: "failed to recover in-flight queue message",
					error,
				});
			}
		}

		if (updates.length > 0) {
			await this.#driver.kvBatchPut(this.#actor.id, updates);
		}
	}

	#scheduleRedelivery(messages: QueueMessage[]): void {
		if (messages.length === 0) {
			return;
		}
		const nextAvailableAt = messages.reduce((min, message) => {
			return message.availableAt < min ? message.availableAt : min;
		}, messages[0].availableAt);

		if (
			this.#redeliveryAt !== undefined &&
			this.#redeliveryAt <= nextAvailableAt
		) {
			return;
		}

		if (this.#redeliveryTimeout) {
			clearTimeout(this.#redeliveryTimeout);
		}

		const delay = Math.max(0, nextAvailableAt - Date.now());
		this.#redeliveryAt = nextAvailableAt;
		this.#redeliveryTimeout = setTimeout(() => {
			this.#redeliveryTimeout = undefined;
			this.#redeliveryAt = undefined;
			void this.#maybeResolveWaiters();
		}, delay);
	}

	#resolveCompletionWaiter(
		messageId: bigint,
		result: QueueCompletionResult,
	): void {
		const waiter = this.#completionWaiters.get(messageId);
		if (!waiter) {
			return;
		}
		this.#completionWaiters.delete(messageId);
		if (waiter.timeoutHandle) {
			clearTimeout(waiter.timeoutHandle);
		}
		waiter.resolve(result);
	}

	#computeBackoffMs(failureCount: number): number {
		const exp = Math.max(0, failureCount - 1);
		const delay = Math.min(BACKOFF_MAX_MS, BACKOFF_INITIAL_MS * 2 ** exp);
		return delay;
	}

	#serializeMetadata(): Uint8Array {
		return QUEUE_METADATA_VERSIONED.serializeWithEmbeddedVersion(
			{
				nextId: this.#metadata.nextId,
				size: this.#metadata.size,
			},
			ACTOR_PERSIST_CURRENT_VERSION,
		);
	}
}
