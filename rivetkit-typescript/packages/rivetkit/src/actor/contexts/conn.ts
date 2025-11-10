import type { Conn } from "../conn/mod";
import type { AnyDatabaseProvider } from "../database";
import type { ActorInstance } from "../instance/mod";
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
