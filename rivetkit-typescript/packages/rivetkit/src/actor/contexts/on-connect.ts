import type { AnyDatabaseProvider } from "../database";
import { ConnInitContext } from "./conn-init";

/**
 * Context for the onConnect lifecycle hook.
 * Called when a connection is successfully established.
 */
export class OnConnectContext<
	TState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider,
> extends ConnInitContext<TState, TVars, TInput, TDatabase> {}
