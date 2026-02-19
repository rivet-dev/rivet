import type { Conn } from "../conn/mod";
import type { AnyDatabaseProvider } from "../database";
import type { ActorDefinition, AnyActorDefinition } from "../definition";
import type { ActorInstance } from "../instance/mod";
import type { SchemaConfig } from "../schema";
import { ConnContext } from "./base/conn";

/**
 * Context for raw WebSocket handlers (onWebSocket).
 */
export class WebSocketContext<
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
> {
	/**
	 * The incoming HTTP request that initiated the WebSocket upgrade.
	 * May be undefined for WebSocket connections initiated without a direct HTTP request.
	 */
	public readonly request: Request | undefined;

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
		conn: Conn<
			TState,
			TConnParams,
			TConnState,
			TVars,
			TInput,
			TDatabase,
			TEvents,
			TQueues
		>,
		request?: Request,
	) {
		super(actor, conn);
		this.request = request;
	}
}

export type WebSocketContextOf<AD extends AnyActorDefinition> =
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
		? WebSocketContext<S, CP, CS, V, I, DB, E, Q>
		: never;
