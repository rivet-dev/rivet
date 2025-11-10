import type { Conn } from "../conn/mod";
import type { AnyDatabaseProvider } from "../database";
import type { ActorInstance } from "../instance/mod";
import { ConnContext } from "./conn";

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
