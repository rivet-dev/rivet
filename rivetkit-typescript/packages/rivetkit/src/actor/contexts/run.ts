import type { AnyDatabaseProvider } from "../database";
import type { ActorDefinition, AnyActorDefinition } from "../definition";
import type { SchemaConfig } from "../schema";
import { ActorContext } from "./base/actor";

/**
 * Context for the run lifecycle hook.
 *
 * This context is passed to the `run` handler which executes after the actor
 * starts. It does not block actor startup and is intended for background tasks.
 *
 * Use `c.aborted` (or `c.abortSignal`) to detect when the actor is stopping and gracefully exit.
 */
export class RunContext<
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider,
	TEvents extends SchemaConfig = Record<never, never>,
	TQueues extends SchemaConfig = Record<never, never>,
> extends ActorContext<
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase,
	TEvents,
	TQueues
> {}

export type RunContextOf<AD extends AnyActorDefinition> =
	AD extends ActorDefinition<
		infer S,
		infer CP,
		infer CS,
		infer V,
		infer I,
		infer DB extends AnyDatabaseProvider,
		infer E extends SchemaConfig,
		infer Q extends SchemaConfig,
		any
	>
		? RunContext<S, CP, CS, V, I, DB, E, Q>
		: never;
