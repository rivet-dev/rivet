import type { AnyDatabaseProvider } from "../database";
import * as errors from "../errors";
import type { QueueManager, QueueMessage as QueueMessageRecord } from "./queue-manager";

/** Options for receiving messages from the queue. */
export interface QueueReceiveOptions {
	/** Maximum number of messages to receive. Defaults to 1. */
	count?: number;
	/** Timeout in milliseconds to wait for messages. Waits indefinitely if not specified. */
	timeout?: number;
	/** When true, message must be manually completed. */
	wait?: boolean;
}

/** Request object for receiving messages from the queue. */
export interface QueueReceiveRequest extends QueueReceiveOptions {
	/** Queue name or names to receive from. */
	name: string | string[];
}

/** User-facing queue interface exposed on ActorContext. */
export class ActorQueue<S, CP, CS, V, I, DB extends AnyDatabaseProvider> {
	#queueManager: QueueManager<S, CP, CS, V, I, DB>;
	#abortSignal: AbortSignal;

	constructor(
		queueManager: QueueManager<S, CP, CS, V, I, DB>,
		abortSignal: AbortSignal,
	) {
		this.#queueManager = queueManager;
		this.#abortSignal = abortSignal;
	}

	/** Receives the next message from a single queue. Returns undefined if no message available. */
	next(
		name: string,
		opts?: QueueReceiveOptions,
	): Promise<QueueMessage | undefined>;
	/** Receives messages from multiple queues. Returns messages matching any of the queue names. */
	next(
		name: string[],
		opts?: QueueReceiveOptions,
	): Promise<QueueMessage[] | undefined>;
	/** Receives messages using a request object for full control over options. */
	next(request: QueueReceiveRequest): Promise<QueueMessage[] | undefined>;
	async next(
		nameOrRequest: string | string[] | QueueReceiveRequest,
		opts: QueueReceiveOptions = {},
	): Promise<QueueMessage | QueueMessage[] | undefined> {
		const request =
			typeof nameOrRequest === "object" && !Array.isArray(nameOrRequest)
				? nameOrRequest
				: { name: nameOrRequest };
		const mergedOptions = request === nameOrRequest ? request : opts;
		const names = Array.isArray(request.name)
			? request.name
			: [request.name];
		const count = mergedOptions.count ?? 1;

		const messages = await this.#queueManager.receive(
			names,
			count,
			mergedOptions.timeout,
			this.#abortSignal,
			mergedOptions.wait ?? false,
		);

		if (Array.isArray(request.name)) {
			return messages?.map((message) =>
				this.#toQueueMessage(message, mergedOptions.wait ?? false),
			);
		}

		if (!messages || messages.length === 0) {
			return undefined;
		}

		return this.#toQueueMessage(messages[0], mergedOptions.wait ?? false);
	}

	#toQueueMessage(
		message: QueueMessageRecord,
		wait: boolean,
	): QueueMessage {
		const base: QueueMessage = {
			id: message.id.toString(),
			name: message.name,
			body: message.body,
			complete: async (data?: unknown) => {
				if (!wait) {
					throw new errors.QueueCompleteNotAllowed();
				}
				await this.#queueManager.complete(message, data);
			},
		};

		return base;
	}

	/** Sends a message to the specified queue. */
	async send(name: string, body: unknown): Promise<QueueMessage> {
		const message = await this.#queueManager.enqueue(name, body);
		return this.#toQueueMessage(message, false);
	}
}

export interface QueueMessage<T = unknown> {
	name: string;
	body: T;
	id: string;
	complete(data?: unknown): Promise<void>;
}
