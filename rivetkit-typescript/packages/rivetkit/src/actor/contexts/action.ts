import type { Conn } from "../conn/mod";
import type { AnyDatabaseProvider } from "../database";
import type { ActorDefinition, AnyActorDefinition } from "../definition";
import type { ActorInstance } from "../instance/mod";
import type { SchemaConfig } from "../schema";
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

/**
 * Extracts the ActionContext type from an ActorDefinition.
 */
export type ActionContextOf<AD extends AnyActorDefinition> =
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
		? ActionContext<S, CP, CS, V, I, DB, E, Q>
		: never;
