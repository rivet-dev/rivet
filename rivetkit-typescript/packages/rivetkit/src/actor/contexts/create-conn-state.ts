import type { AnyDatabaseProvider } from "../database";
import { ConnInitContext } from "./conn-init";

/**
 * Context for the createConnState lifecycle hook.
 * Called to initialize connection-specific state when a connection is created.
 */
export class CreateConnStateContext<
	TState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider,
> extends ConnInitContext<TState, TVars, TInput, TDatabase> {}
