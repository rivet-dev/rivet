import { Effect } from "effect";
import type { ActorContext } from "rivetkit";

export interface QueueReceiveOptions {
	count?: number;
	timeout?: number;
}

export interface QueueMessage {
	id: bigint;
	name: string;
	body: unknown;
	createdAt: number;
}

type AnyQueue = {
	next: (
		nameOrNames: string | string[],
		opts?: QueueReceiveOptions,
	) => Promise<QueueMessage | QueueMessage[] | undefined>;
};

/**
 * Receives the next message from a single queue.
 * Returns undefined if no message available or timeout reached.
 */
export const next = <TState, TConnParams, TConnState, TVars, TInput>(
	c: ActorContext<TState, TConnParams, TConnState, TVars, TInput, undefined>,
	name: string,
	opts?: QueueReceiveOptions,
): Effect.Effect<QueueMessage | undefined, never, never> =>
	Effect.promise(
		() =>
			(c as unknown as { queue: AnyQueue }).queue.next(name, opts) as Promise<
				QueueMessage | undefined
			>,
	);

/**
 * Receives messages from multiple queues.
 * Returns messages matching any of the queue names.
 */
export const nextMultiple = <TState, TConnParams, TConnState, TVars, TInput>(
	c: ActorContext<TState, TConnParams, TConnState, TVars, TInput, undefined>,
	names: string[],
	opts?: QueueReceiveOptions,
): Effect.Effect<QueueMessage[] | undefined, never, never> =>
	Effect.promise(
		() =>
			(c as unknown as { queue: AnyQueue }).queue.next(names, opts) as Promise<
				QueueMessage[] | undefined
			>,
	);
