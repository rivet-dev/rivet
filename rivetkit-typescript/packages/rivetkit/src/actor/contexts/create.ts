import type { AnyDatabaseProvider } from "../database";
import type { ActorDefinition, AnyActorDefinition } from "../definition";
import type { SchemaConfig } from "../schema";
import { ActorContext } from "./base/actor";

/**
 * Context for the onCreate lifecycle hook.
 */
export class CreateContext<
	TState,
	TInput,
	TDatabase extends AnyDatabaseProvider,
	TEvents extends SchemaConfig = Record<never, never>,
	TQueues extends SchemaConfig = Record<never, never>,
> extends ActorContext<
	TState,
	never,
	never,
	never,
	TInput,
	TDatabase,
	TEvents,
	TQueues
> {}

export type CreateContextOf<AD extends AnyActorDefinition> =
	AD extends ActorDefinition<
		infer S,
		any,
		any,
		any,
		infer I,
		infer DB extends AnyDatabaseProvider,
		infer E extends SchemaConfig,
		infer Q extends SchemaConfig,
		any
	>
		? CreateContext<S, I, DB, E, Q>
		: never;
