import type { AnyDatabaseProvider } from "../database";
import type { ActorDefinition, AnyActorDefinition } from "../definition";
import { ActorContext } from "./base/actor";

/**
 * Context for the createVars lifecycle hook.
 */
export class CreateVarsContext<
	TState,
	TInput,
	TDatabase extends AnyDatabaseProvider,
> extends ActorContext<TState, never, never, never, TInput, TDatabase> {}


export type CreateVarsContextOf<AD extends AnyActorDefinition> =
	AD extends ActorDefinition<
		infer S,
		any,
		any,
		any,
		infer I,
		infer DB extends AnyDatabaseProvider,
		any
	>
		? CreateVarsContext<S, I, DB>
		: never;
