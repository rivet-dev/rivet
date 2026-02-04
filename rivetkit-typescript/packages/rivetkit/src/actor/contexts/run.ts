import type { AnyDatabaseProvider } from "../database";
import type { ActorDefinition, AnyActorDefinition } from "../definition";
import { ActorContext } from "./base/actor";

/**
 * Context for the run lifecycle hook.
 *
 * This context is passed to the `run` handler which executes after the actor
 * starts. It does not block actor startup and is intended for background tasks.
 *
 * Use `c.abortSignal` to detect when the actor is stopping and gracefully exit.
 */
export class RunContext<
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

export type RunContextOf<AD extends AnyActorDefinition> =
	AD extends ActorDefinition<
		infer S,
		infer CP,
		infer CS,
		infer V,
		infer I,
		infer DB extends AnyDatabaseProvider,
		any
	>
		? RunContext<S, CP, CS, V, I, DB>
		: never;
