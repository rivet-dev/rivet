import type { Conn } from "../conn/mod";
import type { AnyDatabaseProvider } from "../database";
import type { ActorDefinition, AnyActorDefinition } from "../definition";
import type { ActorInstance } from "../instance/mod";
import { ConnContext } from "./base/conn";

/**
 * Context for a remote procedure call.
 */
export class ActionContext<
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider,
> extends ConnContext<
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase
> {}

/**
 * Extracts the ActionContext type from an ActorDefinition.
 */
export type ActionContextOf<AD extends AnyActorDefinition> = AD extends ActorDefinition<
	infer S,
	infer CP,
	infer CS,
	infer V,
	infer I,
	infer DB extends AnyDatabaseProvider,
	any
>
	? ActionContext<S, CP, CS, V, I, DB>
	: never;
