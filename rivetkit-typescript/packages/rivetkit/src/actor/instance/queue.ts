import type { AnyDatabaseProvider } from "../database";
import type { InferSchemaMap, SchemaConfig } from "../schema";
import type { QueueManager, QueueMessage } from "./queue-manager";

/** Options for receiving messages from the queue. */
export interface QueueReceiveOptions {
	/** Maximum number of messages to receive. Defaults to 1. */
	count?: number;
	/** Timeout in milliseconds to wait for messages. Waits indefinitely if not specified. */
	timeout?: number;
}

/** Request object for receiving messages from the queue. */
export interface QueueReceiveRequest extends QueueReceiveOptions {
	/** Queue name or names to receive from. */
	name: string | string[];
}

export type QueueMessageOf<Body> = Omit<QueueMessage, "body"> & {
	body: Body;
};

/** User-facing queue interface exposed on ActorContext. */
export class ActorQueue<
	S,
	CP,
	CS,
	V,
	I,
	DB extends AnyDatabaseProvider,
	TEvents extends SchemaConfig = Record<never, never>,
	TQueues extends SchemaConfig = Record<never, never>,
> {
	#queueManager: QueueManager<S, CP, CS, V, I, DB, TEvents, TQueues>;
	#abortSignal: AbortSignal;

	constructor(
		queueManager: QueueManager<S, CP, CS, V, I, DB, TEvents, TQueues>,
		abortSignal: AbortSignal,
	) {
		this.#queueManager = queueManager;
		this.#abortSignal = abortSignal;
	}

	/** Receives the next message from a single queue. Returns undefined if no message available. */
	next<K extends keyof TQueues & string>(
		name: K,
		opts?: QueueReceiveOptions,
	): Promise<QueueMessageOf<InferSchemaMap<TQueues>[K]> | undefined>;
	next(
		name: keyof TQueues extends never ? string : never,
		opts?: QueueReceiveOptions,
	): Promise<QueueMessage | undefined>;
	/** Receives messages from multiple queues. Returns messages matching any of the queue names. */
	next<K extends keyof TQueues & string>(
		name: K[],
		opts?: QueueReceiveOptions,
	): Promise<Array<QueueMessageOf<InferSchemaMap<TQueues>[K]>> | undefined>;
	next(
		name: keyof TQueues extends never ? string[] : never,
		opts?: QueueReceiveOptions,
	): Promise<QueueMessage[] | undefined>;
	/** Receives messages using a request object for full control over options. */
	next<K extends keyof TQueues & string>(
		request: QueueReceiveRequest & { name: K },
	): Promise<QueueMessageOf<InferSchemaMap<TQueues>[K]> | undefined>;
	next<K extends keyof TQueues & string>(
		request: QueueReceiveRequest & { name: K[] },
	): Promise<Array<QueueMessageOf<InferSchemaMap<TQueues>[K]>> | undefined>;
	next(
		request: QueueReceiveRequest & {
			name: keyof TQueues extends never ? string | string[] : never;
		},
	): Promise<QueueMessage[] | undefined>;
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
		);

		if (Array.isArray(request.name)) {
			return messages;
		}

		if (!messages || messages.length === 0) {
			return undefined;
		}

		return messages[0];
	}

	/** Sends a message to the specified queue. */
	send<K extends keyof TQueues & string>(
		name: K,
		body: InferSchemaMap<TQueues>[K],
	): Promise<QueueMessage>;
	send(
		name: keyof TQueues extends never ? string : never,
		body: unknown,
	): Promise<QueueMessage>;
	async send(name: string, body: unknown): Promise<QueueMessage> {
		return await this.#queueManager.enqueue(name, body);
	}
}
