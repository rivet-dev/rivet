import type { AnyDatabaseProvider } from "../database";
import type { ActorDefinition, AnyActorDefinition } from "../definition";
import type { SchemaConfig } from "../schema";
import { ConnInitContext } from "./base/conn-init";

/**
 * Context for the onBeforeConnect lifecycle hook.
 */
export class BeforeConnectContext<
	TState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider,
	TEvents extends SchemaConfig = Record<never, never>,
	TQueues extends SchemaConfig = Record<never, never>,
> extends ConnInitContext<TState, TVars, TInput, TDatabase, TEvents, TQueues> {}

export type BeforeConnectContextOf<AD extends AnyActorDefinition> =
	AD extends ActorDefinition<
		infer S,
		any,
		any,
		infer V,
		infer I,
		infer DB extends AnyDatabaseProvider,
		infer E extends SchemaConfig,
		infer Q extends SchemaConfig,
		any
	>
		? BeforeConnectContext<S, V, I, DB, E, Q>
		: never;
