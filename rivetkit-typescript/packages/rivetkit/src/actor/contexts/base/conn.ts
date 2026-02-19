import type { Conn } from "../../conn/mod";
import type { AnyDatabaseProvider } from "../../database";
import type { ActorDefinition, AnyActorDefinition } from "../../definition";
import type { ActorInstance } from "../../instance/mod";
import type { SchemaConfig } from "../../schema";
import { ActorContext } from "./actor";

/**
 * Base context for connection-based handlers.
 * Extends ActorContext with connection-specific functionality.
 */
export abstract class ConnContext<
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
> {
	/**
	 * @internal
	 */
	constructor(
		actor: ActorInstance<
			TState,
			TConnParams,
			TConnState,
			TVars,
			TInput,
			TDatabase,
			TEvents,
			TQueues
		>,
		public readonly conn: Conn<
			TState,
			TConnParams,
			TConnState,
			TVars,
			TInput,
			TDatabase,
			TEvents,
			TQueues
		>,
	) {
		super(actor);
	}
}

export type ConnContextOf<AD extends AnyActorDefinition> =
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
		? ConnContext<S, CP, CS, V, I, DB, E, Q>
		: never;
