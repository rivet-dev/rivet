import type { AnyDatabaseProvider } from "../database";
import { ConnInitContext } from "./conn-init";

/**
 * Context for the onBeforeConnect lifecycle hook.
 * Called before a connection is established, allowing for validation and early rejection.
 */
export class OnBeforeConnectContext<
	TState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider,
> extends ConnInitContext<TState, TVars, TInput, TDatabase> {}
