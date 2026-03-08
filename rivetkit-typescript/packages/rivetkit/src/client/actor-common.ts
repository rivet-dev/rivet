import type { ActorDefinition, AnyActorDefinition } from "@/actor/definition";
import type {
	EventSchemaConfig,
	InferEventArgs,
	InferQueueCompleteMap,
	InferSchemaMap,
	QueueSchemaConfig,
} from "@/actor/schema";
import type {
	QueueSendNoWaitOptions,
	QueueSendResult,
	QueueSendWaitOptions,
} from "./queue";

/**
 * Action function returned by Actor connections and handles.
 *
 * @typedef {Function} ActorActionFunction
 * @template Args
 * @template Response
 * @param {...Args} args - Arguments for the action function.
 * @returns {Promise<Response>}
 */
export type ActorActionFunction<
	Args extends Array<unknown> = unknown[],
	Response = unknown,
> = (
	...args: Args extends [unknown, ...infer Rest] ? Rest : Args
) => Promise<Response>;

/**
 * Maps action methods from actor definition to typed function signatures.
 */
export type ActorDefinitionActions<AD extends AnyActorDefinition> =
	// biome-ignore lint/suspicious/noExplicitAny: safe to use any here
	AD extends ActorDefinition<any, any, any, any, any, any, any, any, infer R>
		? {
				[K in keyof R]: R[K] extends (
					...args: infer Args
				) => infer Return
					? ActorActionFunction<Args, Return>
					: never;
			}
		: never;

type ActorQueueSend<TQueues extends QueueSchemaConfig> = {
	<K extends keyof TQueues & string>(
		name: K,
		body: InferSchemaMap<TQueues>[K],
		options: QueueSendWaitOptions,
	): Promise<QueueSendResult<InferQueueCompleteMap<TQueues>[K]>>;
	<K extends keyof TQueues & string>(
		name: K,
		body: InferSchemaMap<TQueues>[K],
		options?: QueueSendNoWaitOptions,
	): Promise<void>;
	(
		name: keyof TQueues extends never ? string : never,
		body: unknown,
		options: QueueSendWaitOptions,
	): Promise<QueueSendResult>;
	(
		name: keyof TQueues extends never ? string : never,
		body: unknown,
		options?: QueueSendNoWaitOptions,
	): Promise<void>;
};

type ActorEventSubscribe<TEvents extends EventSchemaConfig> = {
	<K extends keyof TEvents & string>(
		eventName: K,
		callback: (...args: InferEventArgs<InferSchemaMap<TEvents>[K]>) => void,
	): () => void;
	(
		eventName: keyof TEvents extends never ? string : never,
		callback: (...args: any[]) => void,
	): () => void;
};

export type ActorDefinitionQueueSend<AD extends AnyActorDefinition> =
	// biome-ignore lint/suspicious/noExplicitAny: safe to use any here
	AD extends ActorDefinition<any, any, any, any, any, any, any, infer Q, any>
		? Q extends QueueSchemaConfig
			? { send: ActorQueueSend<Q> }
			: never
		: never;

export type ActorDefinitionEventSubscriptions<AD extends AnyActorDefinition> =
	// biome-ignore lint/suspicious/noExplicitAny: safe to use any here
	AD extends ActorDefinition<any, any, any, any, any, any, infer E, any, any>
		? E extends EventSchemaConfig
			? {
					on: ActorEventSubscribe<E>;
					once: ActorEventSubscribe<E>;
				}
			: never
		: never;
