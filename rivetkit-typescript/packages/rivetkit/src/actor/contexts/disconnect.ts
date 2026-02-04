import type { Conn } from "../conn/mod";
import type { AnyDatabaseProvider } from "../database";
import type { ActorDefinition, AnyActorDefinition } from "../definition";
import type { SchemaConfig } from "../schema";
import { ActorContext } from "./base/actor";

/**
 * Context for the onDisconnect lifecycle hook.
 */
export class DisconnectContext<
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

export type DisconnectContextOf<AD extends AnyActorDefinition> =
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
		? DisconnectContext<S, CP, CS, V, I, DB, E, Q>
		: never;
