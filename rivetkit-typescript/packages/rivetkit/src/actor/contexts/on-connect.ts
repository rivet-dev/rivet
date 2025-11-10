import type { AnyDatabaseProvider } from "../database";
import { ConnContext } from "./conn";

/**
 * Context for the onConnect lifecycle hook.
 * Called when a connection is successfully established.
 */
export class OnConnectContext<
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
