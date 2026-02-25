import * as errors from "../errors";
import type { AnyDatabaseProvider } from "../database";
import type {
	EventSchemaConfig,
	InferQueueCompleteMap,
	InferSchemaMap,
	QueueSchemaConfig,
} from "../schema";
import { joinAbortSignals } from "../utils";
import type { QueueManager, QueueMessage } from "./queue-manager";

export type QueueMessageOf<Name extends string, Body> = Omit<
	QueueMessage,
	"name" | "body"
> & {
	name: Name;
	body: Body;
};

export type QueueName<TQueues extends QueueSchemaConfig> = keyof TQueues & string;
export type QueueFilterName<TQueues extends QueueSchemaConfig> =
	keyof TQueues extends never ? string : QueueName<TQueues>;

type QueueMessageForName<
	TQueues extends QueueSchemaConfig,
	TName extends QueueFilterName<TQueues>,
> = keyof TQueues extends never
	? QueueMessage
	: TName extends QueueName<TQueues>
		? QueueMessageOf<TName, InferSchemaMap<TQueues>[TName]>
		: never;

type QueueCompleteArgs<T> = undefined extends T
	? [response?: T]
	: [response: T];

type QueueCompleteArgsForName<
	TQueues extends QueueSchemaConfig,
	TName extends QueueFilterName<TQueues>,
> = keyof TQueues extends never
	? [response?: unknown]
	: TName extends QueueName<TQueues>
		? [InferQueueCompleteMap<TQueues>[TName]] extends [never]
			? [response?: unknown]
			: QueueCompleteArgs<InferQueueCompleteMap<TQueues>[TName]>
		: [response?: unknown];

type QueueCompletableMessageForName<
	TQueues extends QueueSchemaConfig,
	TName extends QueueFilterName<TQueues>,
> = QueueMessageForName<TQueues, TName> & {
	complete(
		...args: QueueCompleteArgsForName<TQueues, TName>
	): Promise<void>;
};

export type QueueResultMessageForName<
	TQueues extends QueueSchemaConfig,
	TName extends QueueFilterName<TQueues>,
	TCompletable extends boolean,
> = TCompletable extends true
	? QueueCompletableMessageForName<TQueues, TName>
	: QueueMessageForName<TQueues, TName>;

/** Options for receiving queue messages. */
export interface QueueNextOptions<
	TName extends string = string,
	TCompletable extends boolean = boolean,
> {
	/** Queue names to receive from. If omitted, reads from all queue names. */
	names?: readonly TName[];
	/** Timeout in milliseconds. Omit to wait indefinitely. */
	timeout?: number;
	/** Optional abort signal for this receive call. */
	signal?: AbortSignal;
	/** Whether to return completable messages. */
	completable?: TCompletable;
}

/** Options for receiving queue message batches. */
export interface QueueNextBatchOptions<
	TName extends string = string,
	TCompletable extends boolean = boolean,
> {
	/** Queue names to receive from. If omitted, reads from all queue names. */
	names?: readonly TName[];
	/** Maximum number of messages to receive. Defaults to 1. */
	count?: number;
	/** Timeout in milliseconds. Omit to wait indefinitely. */
	timeout?: number;
	/** Optional abort signal for this receive call. */
	signal?: AbortSignal;
	/** Whether to return completable messages. */
	completable?: TCompletable;
}

/** Options for non-blocking queue reads. */
export interface QueueTryNextOptions<
	TName extends string = string,
	TCompletable extends boolean = boolean,
> {
	/** Queue names to receive from. If omitted, reads from all queue names. */
	names?: readonly TName[];
	/** Whether to return completable messages. */
	completable?: TCompletable;
}

/** Options for non-blocking queue batch reads. */
export interface QueueTryNextBatchOptions<
	TName extends string = string,
	TCompletable extends boolean = boolean,
> {
	/** Queue names to receive from. If omitted, reads from all queue names. */
	names?: readonly TName[];
	/** Maximum number of messages to receive. Defaults to 1. */
	count?: number;
	/** Whether to return completable messages. */
	completable?: TCompletable;
}

/** Options for queue async iteration. */
export interface QueueIterOptions<
	TName extends string = string,
	TCompletable extends boolean = boolean,
> {
	/** Queue names to receive from. If omitted, reads from all queue names. */
	names?: readonly TName[];
	/** Optional abort signal for this iterator. */
	signal?: AbortSignal;
	/** Whether to return completable messages. */
	completable?: TCompletable;
}

/** User-facing queue interface exposed on ActorContext. */
export class ActorQueue<
	S,
	CP,
	CS,
	V,
	I,
	DB extends AnyDatabaseProvider,
	TEvents extends EventSchemaConfig = Record<never, never>,
	TQueues extends QueueSchemaConfig = Record<never, never>,
> {
	#queueManager: QueueManager<S, CP, CS, V, I, DB, TEvents, TQueues>;
	#abortSignal: AbortSignal;
	#pendingCompletableMessageIds = new Set<string>();

	constructor(
		queueManager: QueueManager<S, CP, CS, V, I, DB, TEvents, TQueues>,
		abortSignal: AbortSignal,
	) {
		this.#queueManager = queueManager;
		this.#abortSignal = abortSignal;
	}

	async next<
		const TName extends QueueFilterName<TQueues>,
		const TCompletable extends boolean = false,
	>(
		opts?: QueueNextOptions<TName, TCompletable>,
	): Promise<QueueResultMessageForName<TQueues, TName, TCompletable> | undefined> {
		const resolvedOpts = (opts ?? {}) as QueueNextOptions<
			TName,
			TCompletable
		>;
		const messages = await this.nextBatch<TName, TCompletable>({
			...(resolvedOpts as QueueNextBatchOptions<TName, TCompletable>),
			count: 1,
		});
		return messages[0];
	}

	async nextBatch<
		const TName extends QueueFilterName<TQueues>,
		const TCompletable extends boolean = false,
	>(
		opts?: QueueNextBatchOptions<TName, TCompletable>,
	): Promise<Array<QueueResultMessageForName<TQueues, TName, TCompletable>>> {
		const resolvedOpts = (opts ?? {}) as QueueNextBatchOptions<
			TName,
			TCompletable
		>;
		const completable = resolvedOpts.completable === true;

		if (this.#pendingCompletableMessageIds.size > 0) {
			throw new errors.QueuePreviousMessageNotCompleted();
		}

		const names = this.#normalizeNames(resolvedOpts.names);
		const count = Math.max(1, resolvedOpts.count ?? 1);
		const { signal, cleanup } = joinAbortSignals(
			this.#abortSignal,
			resolvedOpts.signal,
		);
		const messages = await this.#queueManager
			.receive(
				names,
				count,
				resolvedOpts.timeout,
				signal,
				completable,
			)
			.finally(cleanup);
		if (!completable) {
			return messages as Array<
				QueueResultMessageForName<TQueues, TName, TCompletable>
			>;
		}
		return messages.map((message) => this.#makeCompletableMessage(message)) as unknown as Array<
			QueueResultMessageForName<TQueues, TName, TCompletable>
		>;
	}

	async tryNext<
		const TName extends QueueFilterName<TQueues>,
		const TCompletable extends boolean = false,
	>(
		opts?: QueueTryNextOptions<TName, TCompletable>,
	): Promise<QueueResultMessageForName<TQueues, TName, TCompletable> | undefined> {
		const resolvedOpts = (opts ?? {}) as QueueTryNextOptions<
			TName,
			TCompletable
		>;
		const messages = await this.tryNextBatch<TName, TCompletable>({
			...(resolvedOpts as QueueTryNextBatchOptions<TName, TCompletable>),
			count: 1,
		});
		return messages[0];
	}

	async tryNextBatch<
		const TName extends QueueFilterName<TQueues>,
		const TCompletable extends boolean = false,
	>(
		opts?: QueueTryNextBatchOptions<TName, TCompletable>,
	): Promise<Array<QueueResultMessageForName<TQueues, TName, TCompletable>>> {
		const resolvedOpts = (opts ?? {}) as QueueTryNextBatchOptions<
			TName,
			TCompletable
		>;
		if (resolvedOpts.completable === true) {
			return (await this.nextBatch<TName, true>({
				names: resolvedOpts.names,
				count: resolvedOpts.count,
				timeout: 0,
				completable: true,
			})) as Array<QueueResultMessageForName<TQueues, TName, TCompletable>>;
		}
		return (await this.nextBatch<TName, false>({
			names: resolvedOpts.names,
			count: resolvedOpts.count,
			timeout: 0,
		})) as Array<QueueResultMessageForName<TQueues, TName, TCompletable>>;
	}

	async *iter<
		const TName extends QueueFilterName<TQueues>,
		const TCompletable extends boolean = false,
	>(
		opts?: QueueIterOptions<TName, TCompletable>,
	): AsyncIterableIterator<
		QueueResultMessageForName<TQueues, TName, TCompletable>
	> {
		const resolvedOpts = (opts ?? {}) as QueueIterOptions<
			TName,
			TCompletable
		>;
		while (!this.#abortSignal.aborted) {
			try {
				const message = resolvedOpts.completable === true
					? await this.next<TName, true>({
							names: resolvedOpts.names,
							signal: resolvedOpts.signal,
							completable: true,
						})
					: await this.next<TName, false>({
							names: resolvedOpts.names,
							signal: resolvedOpts.signal,
						});
				if (!message) {
					continue;
				}
				yield message as QueueResultMessageForName<
					TQueues,
					TName,
					TCompletable
				>;
			} catch (error) {
				if (error instanceof errors.ActorAborted) {
					return;
				}
				throw error;
			}
		}
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

	#normalizeNames(names: readonly string[] | undefined): string[] | undefined {
		if (!names || names.length === 0) {
			return undefined;
		}
		return [...new Set(names)];
	}

	#makeCompletableMessage(
		message: QueueMessage,
	): QueueMessage & {
		complete: (response?: unknown) => Promise<void>;
	} {
		const messageId = message.id.toString();
		this.#pendingCompletableMessageIds.add(messageId);

		let completed = false;
		const completableMessage = {
			...message,
			complete: async (response?: unknown) => {
				if (completed) {
					throw new errors.QueueAlreadyCompleted();
				}
				completed = true;
				try {
					await this.#queueManager.completeMessage(message, response);
					this.#pendingCompletableMessageIds.delete(messageId);
				} catch (error) {
					completed = false;
					throw error;
				}
			},
		};
		return completableMessage;
	}
}
