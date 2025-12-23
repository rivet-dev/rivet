import type { Conn } from "../../conn/mod";
import type { AnyDatabaseProvider } from "../../database";
import type {
	ActorDefinition,
	AnyActorDefinition,
} from "../../definition";
import type { ActorInstance } from "../../instance/mod";
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
> extends ActorContext<
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase
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
			TDatabase
		>,
		public readonly conn: Conn<
			TState,
			TConnParams,
			TConnState,
			TVars,
			TInput,
			TDatabase
		>,
	) {
		super(actor);
	}
}

export type ConnContextOf<AD extends AnyActorDefinition> = AD extends ActorDefinition<
	infer S,
	infer CP,
	infer CS,
	infer V,
	infer I,
	infer DB extends AnyDatabaseProvider,
	any
>
	? ConnContext<S, CP, CS, V, I, DB>
	: never;
