import type { AnyDatabaseProvider } from "../database";
import type { ActorDefinition, AnyActorDefinition } from "../definition";
import type { EventSchemaConfig, QueueSchemaConfig } from "../schema";
import { ActorContext } from "./base/actor";

/**
 * Context for the createVars lifecycle hook.
 */
export class CreateVarsContext<
	TState,
	TInput,
	TDatabase extends AnyDatabaseProvider,
	TEvents extends EventSchemaConfig = Record<never, never>,
	TQueues extends QueueSchemaConfig = Record<never, never>,
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

export type CreateVarsContextOf<AD extends AnyActorDefinition> =
	AD extends ActorDefinition<
		infer S,
		any,
		any,
		any,
		infer I,
		infer DB extends AnyDatabaseProvider,
		infer E extends EventSchemaConfig,
		infer Q extends QueueSchemaConfig,
		any
	>
		? CreateVarsContext<S, I, DB, E, Q>
		: never;
