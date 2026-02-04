import type { ActorDefinition, AnyActorDefinition } from "@/actor/definition";

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
