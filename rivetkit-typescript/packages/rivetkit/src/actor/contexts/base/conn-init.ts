import type { AnyDatabaseProvider } from "../../database";
import type { ActorDefinition, AnyActorDefinition } from "../../definition";
import type { ActorInstance } from "../../instance/mod";
import type { EventSchemaConfig, QueueSchemaConfig } from "../../schema";
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
	TEvents extends EventSchemaConfig = Record<never, never>,
	TQueues extends QueueSchemaConfig = Record<never, never>,
> extends ActorContext<
	TState,
	never,
	never,
	TVars,
	TInput,
	TDatabase,
	TEvents,
	TQueues
> {
	/**
	 * The incoming request that initiated the connection.
	 * May be undefined for connections initiated without a direct HTTP request.
	 */
	public readonly request: Request | undefined;

	/**
	 * @internal
	 */
	constructor(
		actor: ActorInstance<
			TState,
			any,
			any,
			TVars,
			TInput,
			TDatabase,
			TEvents,
			TQueues
		>,
		request: Request | undefined,
	) {
		super(actor as any);
		this.request = request;
	}
}

export type ConnInitContextOf<AD extends AnyActorDefinition> =
	AD extends ActorDefinition<
		infer S,
		any,
		any,
		infer V,
		infer I,
		infer DB extends AnyDatabaseProvider,
		infer E extends EventSchemaConfig,
		infer Q extends QueueSchemaConfig,
		any
	>
		? ConnInitContext<S, V, I, DB, E, Q>
		: never;
