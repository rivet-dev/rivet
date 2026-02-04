import type { AnyDatabaseProvider } from "../database";
import type { ActorDefinition, AnyActorDefinition } from "../definition";
import type { SchemaConfig } from "../schema";
import { ConnContext } from "./base/conn";

/**
 * Context for the onConnect lifecycle hook.
 */
export class ConnectContext<
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider,
	TEvents extends SchemaConfig = Record<never, never>,
	TQueues extends SchemaConfig = Record<never, never>,
> extends ConnContext<
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase,
	TEvents,
	TQueues
> {}

export type ConnectContextOf<AD extends AnyActorDefinition> =
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
		? ConnectContext<S, CP, CS, V, I, DB, E, Q>
		: never;
