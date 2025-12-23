import type { Conn } from "../conn/mod";
import type { ActorDefinition, AnyActorDefinition } from "../definition";
import type { AnyDatabaseProvider } from "../database";
import type { ActorInstance } from "../instance/mod";
import { ConnContext } from "./base/conn";

/**
 * Context for raw HTTP request handlers (onRequest).
 */
export class RequestContext<
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
> {
	/**
	 * The incoming HTTP request.
	 * May be undefined for request contexts initiated without a direct HTTP request.
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
			TDatabase
		>,
		conn: Conn<TState, TConnParams, TConnState, TVars, TInput, TDatabase>,
		request?: Request,
	) {
		super(actor, conn);
		this.request = request;
	}
}


export type RequestContextOf<AD extends AnyActorDefinition> =
	AD extends ActorDefinition<
		infer S,
		infer CP,
		infer CS,
		infer V,
		infer I,
		infer DB extends AnyDatabaseProvider,
		any
	>
		? RequestContext<S, CP, CS, V, I, DB>
		: never;
