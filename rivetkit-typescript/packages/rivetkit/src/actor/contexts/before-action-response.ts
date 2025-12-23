import type { AnyDatabaseProvider } from "../database";
import type { ActorDefinition, AnyActorDefinition } from "../definition";
import { ActorContext } from "./base/actor";

/**
 * Context for the onBeforeActionResponse lifecycle hook.
 */
export class BeforeActionResponseContext<
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider,
> extends ActorContext<
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase
> {}

export type BeforeActionResponseContextOf<AD extends AnyActorDefinition> =
	AD extends ActorDefinition<
		infer S,
		infer CP,
		infer CS,
		infer V,
		infer I,
		infer DB extends AnyDatabaseProvider,
		any
	>
		? BeforeActionResponseContext<S, CP, CS, V, I, DB>
		: never;
