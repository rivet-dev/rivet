import type { Conn } from "../conn/mod";
import type { ActorDefinition, AnyActorDefinition } from "../definition";
import type { AnyDatabaseProvider } from "../database";
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
> extends ActorContext<
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase
> {}


export type DisconnectContextOf<AD extends AnyActorDefinition> =
	AD extends ActorDefinition<
		infer S,
		infer CP,
		infer CS,
		infer V,
		infer I,
		infer DB extends AnyDatabaseProvider,
		any
	>
		? DisconnectContext<S, CP, CS, V, I, DB>
		: never;
