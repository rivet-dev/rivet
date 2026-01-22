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
}

interface QueueMetadata {
	nextId: bigint;
	size: number;
}

interface QueueWaiter {
	id: string;
	nameSet: Set<string>;
	count: number;
	resolve: (messages: QueueMessage[]) => void;
	reject: (error: Error) => void;
	signal?: AbortSignal;
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

export class QueueManager<S, CP, CS, V, I, DB extends AnyDatabaseProvider> {
	#actor: ActorInstance<S, CP, CS, V, I, DB>;
	#driver: ActorDriver;
	#waiters = new Map<string, QueueWaiter>();
	#metadata: QueueMetadata = { ...DEFAULT_METADATA };
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
		};

		this.#actor.resetSleepTimer();
		await this.#maybeResolveWaiters();
		this.#notifyMessageListeners(name);

		return message;
	}

	/** Receives messages from the queue matching the given names. Waits until messages are available or timeout is reached. */
	async receive(
		names: string[],
		count: number,
		timeout?: number,
		abortSignal?: AbortSignal,
	): Promise<QueueMessage[] | undefined> {
		this.#actor.assertReady();
		const limitedCount = Math.max(1, count);
		const nameSet = new Set(names);

		const immediate = await this.#drainMessages(nameSet, limitedCount);
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
		await this.#removeMessages(toRemove);
		return toRemove.map((entry) => entry.id);
	}

	async #drainMessages(
		nameSet: Set<string>,
		count: number,
	): Promise<QueueMessage[]> {
		if (this.#metadata.size === 0) {
			return [];
		}
		const entries = await this.#loadQueueMessages();
		const matched = entries.filter((entry) => nameSet.has(entry.name));
		if (matched.length === 0) {
			return [];
		}

		const selected = matched.slice(0, count);
		await this.#removeMessages(selected);
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
			if (!listener.nameSet.has(name)) {
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
		this.#metadata.size = Math.max(0, this.#metadata.size - messages.length);

		// Delete messages and update metadata
		// Note: kvBatchDelete doesn't support mixed operations, so we do two calls
		await this.#driver.kvBatchDelete(this.#actor.id, keys);
		await this.#driver.kvBatchPut(this.#actor.id, [
			[KEYS.QUEUE_METADATA, this.#serializeMetadata()],
		]);

		this.#actor.inspector.updateQueueSize(this.#metadata.size);
	}

	async #maybeResolveWaiters() {
		if (this.#waiters.size === 0) {
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
			);
			if (messages.length === 0) {
				continue;
			}
			this.#waiters.delete(waiter.id);
			if (waiter.timeoutHandle) {
				clearTimeout(waiter.timeoutHandle);
			}
			waiter.resolve(messages);
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
