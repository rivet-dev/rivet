import type { AnyDatabaseProvider } from "../database";
import type { ActorDefinition, AnyActorDefinition } from "../definition";
import { ConnInitContext } from "./base/conn-init";

/**
 * Context for the onBeforeConnect lifecycle hook.
 */
export class BeforeConnectContext<
	TState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider,
> extends ConnInitContext<TState, TVars, TInput, TDatabase> {}

export type BeforeConnectContextOf<AD extends AnyActorDefinition> =
	AD extends ActorDefinition<
		infer S,
		any,
		any,
		infer V,
		infer I,
		infer DB extends AnyDatabaseProvider,
		any
	>
		? BeforeConnectContext<S, V, I, DB>
		: never;
