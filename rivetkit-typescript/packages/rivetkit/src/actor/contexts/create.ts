import type { AnyDatabaseProvider } from "../database";
import type { ActorDefinition, AnyActorDefinition } from "../definition";
import { ActorContext } from "./base/actor";

/**
 * Context for the onCreate lifecycle hook.
 */
export class CreateContext<
	TState,
	TInput,
	TDatabase extends AnyDatabaseProvider,
> extends ActorContext<TState, never, never, never, TInput, TDatabase> {}


export type CreateContextOf<AD extends AnyActorDefinition> =
	AD extends ActorDefinition<
		infer S,
		any,
		any,
		any,
		infer I,
		infer DB extends AnyDatabaseProvider,
		any
	>
		? CreateContext<S, I, DB>
		: never;
