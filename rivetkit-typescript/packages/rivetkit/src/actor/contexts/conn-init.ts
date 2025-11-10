import type { AnyDatabaseProvider } from "../database";
import type { ActorInstance } from "../instance/mod";
import { ActorContext } from "./actor";

/**
 * Base context for connection initialization handlers.
 * Extends ActorContext with request-specific functionality for connection lifecycle events.
 */
export abstract class ConnInitContext<
	TState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider,
> extends ActorContext<TState, undefined, undefined, TVars, TInput, TDatabase> {
	/**
	 * The incoming request that initiated the connection.
	 * May be undefined for connections initiated without a direct HTTP request.
	 */
	public readonly request: Request | undefined;

	/**
	 * @internal
	 */
	constructor(
		actor: ActorInstance<TState, any, any, TVars, TInput, TDatabase>,
		request: Request | undefined,
	) {
		super(actor);
		this.request = request;
	}
}
