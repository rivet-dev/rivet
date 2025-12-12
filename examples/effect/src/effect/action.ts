import { Context, Effect } from "effect";
import type {
	ActorContext,
	AnyDatabaseProvider,
	Registry,
	ActionContext,
	InitContext,
} from "rivetkit";
import type { YieldWrap } from "effect/Utils";
import { ActorContextTag } from "./hook";

export * from "./hook";

export const getConn = <
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider,
>(
	c: ActionContext<TState, TConnParams, TConnState, TVars, TInput, TDatabase>,
) => Effect.succeed(c.conn);

export function effect<
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider,
	AEff,
	Args extends unknown[],
>(
	genFn: (
		c: ActorContext<
			TState,
			TConnParams,
			TConnState,
			TVars,
			TInput,
			TDatabase
		>,
		...args: Args
	) => Generator<YieldWrap<Effect.Effect<any, never, any>>, AEff, never>,
): (
	c: ActionContext<TState, TConnParams, TConnState, TVars, TInput, TDatabase>,
	...args: Args
) => Promise<AEff> {
	return (c, ...args) => {
		const gen = genFn(c, ...args);
		const eff = Effect.gen<YieldWrap<Effect.Effect<any, never, any>>, AEff>(
			() => gen,
		);

		// Provide ActorContext via Effect Context
		const withContext = Effect.provideService(
			eff,
			ActorContextTag,
			c,
		) as Effect.Effect<AEff, never, never>;

		return Effect.runPromise(withContext);
	};
}

export function workflow<
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider,
	AEff,
	Args extends unknown[],
>(
	genFn: (
		c: ActorContext<
			TState,
			TConnParams,
			TConnState,
			TVars,
			TInput,
			TDatabase
		>,
		...args: Args
	) => Generator<YieldWrap<Effect.Effect<any, any, any>>, AEff, never>,
): (
	c: ActionContext<TState, TConnParams, TConnState, TVars, TInput, TDatabase>,
	...args: Args
) => Promise<AEff> {
	return (c, ...args) => {
		const gen = genFn(c, ...args);
		const eff = Effect.gen<YieldWrap<Effect.Effect<any, any, any>>, AEff>(
			() => gen,
		);

		// Provide ActorContext via Effect Context
		const withContext = Effect.provideService(
			eff,
			ActorContextTag,
			c,
		) as Effect.Effect<AEff, any, never>;

		// Make workflow execution durable by using waitUntil
		const workflowPromise = Effect.runPromise(withContext);
		c.waitUntil(workflowPromise.then(() => {}));

		return workflowPromise;
	};
}
