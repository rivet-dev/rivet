import { Effect } from "effect";
import type { ActorContext } from "rivetkit";
import { QueueError } from "./errors.ts";
import type { AnyDatabaseProvider } from "./rivet-actor.ts";

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
export const next = <
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider = AnyDatabaseProvider,
>(
	c: ActorContext<TState, TConnParams, TConnState, TVars, TInput, TDatabase>,
	name: string,
	opts?: QueueReceiveOptions,
): Effect.Effect<QueueMessage | undefined, QueueError, never> =>
	Effect.tryPromise({
		try: () =>
			(c as unknown as { queue: AnyQueue }).queue.next(
				name,
				opts,
			) as Promise<QueueMessage | undefined>,
		catch: (error) =>
			new QueueError({
				message: "Failed to receive message from queue",
				cause: error,
			}),
	});

/**
 * Receives messages from multiple queues.
 * Returns messages matching any of the queue names.
 */
export const nextMultiple = <
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider = AnyDatabaseProvider,
>(
	c: ActorContext<TState, TConnParams, TConnState, TVars, TInput, TDatabase>,
	names: string[],
	opts?: QueueReceiveOptions,
): Effect.Effect<QueueMessage[] | undefined, QueueError, never> =>
	Effect.tryPromise({
		try: () =>
			(c as unknown as { queue: AnyQueue }).queue.next(
				names,
				opts,
			) as Promise<QueueMessage[] | undefined>,
		catch: (error) =>
			new QueueError({
				message: "Failed to receive messages from queues",
				cause: error,
			}),
	});
