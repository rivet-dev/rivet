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
import type { EventSchemaConfig, QueueSchemaConfig } from "../schema";
import {
	decodeQueueMessageKey,
	makeQueueMessageKey,
	queueMessagesPrefix,
	queueMetadataKey,
} from "./keys";
import type { ActorInstance } from "./mod";

export interface QueueMessage {
	id: bigint;
	name: string;
	body: unknown;
	createdAt: number;
}

interface QueueMetadata {
	nextId: bigint;
	size: number;
}

interface QueueWaiter {
	id: string;
	nameSet?: Set<string>;
	count: number;
	completable: boolean;
	resolve: (messages: QueueMessage[]) => void;
	reject: (error: Error) => void;
}

interface MessageListener {
	nameSet?: Set<string>;
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

const QUEUE_METADATA_KEY = queueMetadataKey();
const QUEUE_MESSAGES_PREFIX = queueMessagesPrefix();

interface PendingCompletion {
	resolve: (result: {
		status: "completed" | "timedOut";
		response?: unknown;
	}) => void;
	timeoutHandle?: ReturnType<typeof setTimeout>;
}

export class QueueManager<
	S,
	CP,
	CS,
	V,
	I,
	DB extends AnyDatabaseProvider,
	E extends EventSchemaConfig = Record<never, never>,
	Q extends QueueSchemaConfig = Record<never, never>,
> {
	#actor: ActorInstance<S, CP, CS, V, I, DB, E, Q>;
	#driver: ActorDriver;
	#waiters = new Map<string, QueueWaiter>();
	#metadata: QueueMetadata = { ...DEFAULT_METADATA };
	#messageListeners = new Set<MessageListener>();
	#pendingCompletions = new Map<string, PendingCompletion>();

	constructor(
		actor: ActorInstance<S, CP, CS, V, I, DB, E, Q>,
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
			QUEUE_METADATA_KEY,
		]);
		if (!metadataBuffer) {
			await this.#driver.kvBatchPut(this.#actor.id, [
				[QUEUE_METADATA_KEY, this.#serializeMetadata()],
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
	}

	/** Adds a message to the queue with the given name and body. */
	async enqueue(name: string, body: unknown): Promise<QueueMessage> {
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
		const encodedMessage =
			QUEUE_MESSAGE_VERSIONED.serializeWithEmbeddedVersion(
				{
					name,
					body: new Uint8Array(bodyCborBuffer).buffer as ArrayBuffer,
					createdAt: BigInt(createdAt),
					failureCount: null,
					availableAt: null,
					inFlight: null,
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
			[QUEUE_METADATA_KEY, encodedMetadata],
		]);

		this.#actor.inspector.updateQueueSize(this.#metadata.size);

		const message: QueueMessage = {
			id,
			name,
			body,
			createdAt,
		};

		this.#actor.resetSleepTimer();
		await this.#maybeResolveWaiters();
		this.#notifyMessageListeners(name);

		return message;
	}

	/**
	 * Adds a message and waits for completion.
	 */
	async enqueueAndWait(
		name: string,
		body: unknown,
		timeout?: number,
	): Promise<{ status: "completed" | "timedOut"; response?: unknown }> {
		if (timeout !== undefined && timeout <= 0) {
			return { status: "timedOut" };
		}

		const message = await this.enqueue(name, body);
		const messageId = message.id.toString();
		const { promise, resolve } = promiseWithResolvers<{
			status: "completed" | "timedOut";
			response?: unknown;
		}>(() => {});

		const pending: PendingCompletion = { resolve };
		if (timeout !== undefined) {
			pending.timeoutHandle = setTimeout(() => {
				this.#pendingCompletions.delete(messageId);
				resolve({ status: "timedOut" });
			}, timeout);
		}
		this.#pendingCompletions.set(messageId, pending);

		return await promise;
	}

	async completeMessage(
		message: QueueMessage,
		response?: unknown,
	): Promise<void> {
		await this.completeMessageById(message.id, response);
	}

	async completeMessageById(
		messageId: bigint,
		response?: unknown,
	): Promise<void> {
		const messageIdString = messageId.toString();
		const pending = this.#pendingCompletions.get(messageIdString);
		if (pending) {
			if (pending.timeoutHandle) {
				clearTimeout(pending.timeoutHandle);
			}
			this.#pendingCompletions.delete(messageIdString);
			pending.resolve({ status: "completed", response });
		}

		await this.deleteMessagesById([messageId]);
	}

	/** Receives messages from the queue matching the given names. Waits until messages are available or timeout is reached. */
	async receive(
		names: string[] | undefined,
		count: number,
		timeout?: number,
		abortSignal?: AbortSignal,
		completable = false,
	): Promise<QueueMessage[]> {
		this.#actor.assertReady();
		const limitedCount = Math.max(1, count);
		const nameSet =
			names && names.length > 0 ? new Set(names) : undefined;

		const immediate = await this.#drainMessages(
			nameSet,
			limitedCount,
			completable,
		);
		if (immediate.length > 0) {
			return immediate;
		}
		if (timeout === 0) {
			return [];
		}

		const { promise, resolve, reject } = promiseWithResolvers<
			QueueMessage[]
		>(() => {});
		const waiterId = crypto.randomUUID();
		let timeoutHandle: ReturnType<typeof setTimeout> | undefined;
		let cleanedUp = false;
		let actorAbortCleanup: (() => void) | undefined;
		let signalAbortCleanup: (() => void) | undefined;

		const cleanup = () => {
			if (cleanedUp) {
				return;
			}
			cleanedUp = true;
			this.#waiters.delete(waiterId);
			if (timeoutHandle) {
				clearTimeout(timeoutHandle);
				timeoutHandle = undefined;
			}
			actorAbortCleanup?.();
			signalAbortCleanup?.();
			this.#actor.endQueueWait();
		};
		const resolveWaiter = (messages: QueueMessage[]) => {
			cleanup();
			resolve(messages);
		};
		const rejectWaiter = (error: Error) => {
			cleanup();
			reject(error);
		};

		const waiter: QueueWaiter = {
			id: waiterId,
			nameSet,
			count: limitedCount,
			completable,
			resolve: resolveWaiter,
			reject: rejectWaiter,
		};

		this.#actor.beginQueueWait();

		if (timeout !== undefined) {
			timeoutHandle = setTimeout(() => {
				resolveWaiter([]);
			}, timeout);
		}

		const onAbort = () => {
			rejectWaiter(new errors.ActorAborted());
		};
		const onStop = () => {
			rejectWaiter(new errors.ActorAborted());
		};
		const actorAbortSignal = this.#actor.abortSignal;
		if (actorAbortSignal.aborted) {
			onStop();
			return promise;
		}
		actorAbortSignal.addEventListener("abort", onStop, { once: true });
		actorAbortCleanup = () =>
			actorAbortSignal.removeEventListener("abort", onStop);

		if (abortSignal) {
			if (abortSignal.aborted) {
				onAbort();
				return promise;
			}
			abortSignal.addEventListener("abort", onAbort, { once: true });
			signalAbortCleanup = () =>
				abortSignal.removeEventListener("abort", onAbort);
		}

		this.#waiters.set(waiterId, waiter);
		return promise;
	}

	async waitForNames(
		names: readonly string[] | undefined,
		abortSignal?: AbortSignal,
	): Promise<void> {
		const nameSet =
			names && names.length > 0 ? new Set(names) : undefined;
		const existing = await this.#loadQueueMessages();
		if (nameSet) {
			if (existing.some((message) => nameSet.has(message.name))) {
				return;
			}
		} else if (existing.length > 0) {
			return;
		}

		return await new Promise<void>((resolve, reject) => {
			this.#actor.beginQueueWait();
			const listener: MessageListener = {
				nameSet,
				resolve: () => {
					this.#removeMessageListener(listener);
					this.#actor.endQueueWait();
					resolve();
				},
				reject: (error) => {
					this.#removeMessageListener(listener);
					this.#actor.endQueueWait();
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
		await this.#removeMessages(toRemove);
		return toRemove.map((entry) => entry.id);
	}

	async #drainMessages(
		nameSet: Set<string> | undefined,
		count: number,
		completable: boolean,
	): Promise<QueueMessage[]> {
		if (this.#metadata.size === 0) {
			return [];
		}
		const entries = await this.#loadQueueMessages();
		const matched = nameSet
			? entries.filter((entry) => nameSet.has(entry.name))
			: entries;
		if (matched.length === 0) {
			return [];
		}

		const selected = matched.slice(0, count);
		if (!completable) {
			await this.#removeMessages(selected);
		}
		const now = Date.now();
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
			QUEUE_MESSAGES_PREFIX,
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
				decoded.push({
					id: messageId,
					name: decodedPayload.name,
					body,
					createdAt: Number(decodedPayload.createdAt),
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
			if (listener.nameSet && !listener.nameSet.has(name)) {
				continue;
			}
			this.#removeMessageListener(listener);
			listener.resolve();
		}
	}

	async #removeMessages(messages: QueueMessage[]): Promise<void> {
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
			[QUEUE_METADATA_KEY, this.#serializeMetadata()],
		]);

		this.#actor.inspector.updateQueueSize(this.#metadata.size);
	}

	async #maybeResolveWaiters() {
		if (this.#waiters.size === 0) {
			return;
		}
		const pending = [...this.#waiters.values()];
		for (const waiter of pending) {
			const messages = await this.#drainMessages(
				waiter.nameSet,
				waiter.count,
				waiter.completable,
			);
			if (messages.length === 0) {
				continue;
			}
			this.#waiters.delete(waiter.id);
			waiter.resolve(messages);
		}
	}

	/** Rebuilds metadata by scanning existing queue messages. Used when metadata is corrupted. */
	async #rebuildMetadata(): Promise<void> {
		const entries = await this.#driver.kvListPrefix(
			this.#actor.id,
			QUEUE_MESSAGES_PREFIX,
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
			[QUEUE_METADATA_KEY, this.#serializeMetadata()],
		]);
		this.#actor.inspector.updateQueueSize(this.#metadata.size);
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
