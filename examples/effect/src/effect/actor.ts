import { Context, Effect } from "effect";
import type {
	ActorContext,
	AnyDatabaseProvider,
	Registry,
} from "rivetkit";
import type { YieldWrap } from "effect/Utils";

// Context tag for accessing ActorContext within Effects
export const ActorContextTag =
	Context.GenericTag<ActorContext<any, any, any, any, any, any>>(
		"ActorContext",
	);

export const context = <
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider,
>(): Effect.Effect<
	ActorContext<TState, TConnParams, TConnState, TVars, TInput, TDatabase>,
	never,
	typeof ActorContextTag
> => ActorContextTag as any;

export const state = <
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider,
>(
	c: ActorContext<TState, TConnParams, TConnState, TVars, TInput, TDatabase>,
): Effect.Effect<TState, never, never> => Effect.succeed(c.state);

export const updateState = <
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider,
>(
	c: ActorContext<TState, TConnParams, TConnState, TVars, TInput, TDatabase>,
	f: (state: TState) => void,
): Effect.Effect<void, never, never> => Effect.sync(() => f(c.state));

export const vars = <
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider,
>(
	c: ActorContext<TState, TConnParams, TConnState, TVars, TInput, TDatabase>,
): Effect.Effect<TVars, never, never> => Effect.succeed(c.vars);

export const updateVars = <
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider,
>(
	c: ActorContext<TState, TConnParams, TConnState, TVars, TInput, TDatabase>,
	f: (vars: TVars) => void,
): Effect.Effect<void, never, never> => Effect.sync(() => f(c.vars));

export const broadcast = <
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider,
	Args extends Array<unknown>,
>(
	c: ActorContext<TState, TConnParams, TConnState, TVars, TInput, TDatabase>,
	name: string,
	...args: Args
): Effect.Effect<void, never, never> =>
	Effect.sync(() => c.broadcast(name, ...args));

export const getLog = <
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider,
>(
	c: ActorContext<TState, TConnParams, TConnState, TVars, TInput, TDatabase>,
) => Effect.succeed(c.log);

export const getActorId = <
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider,
>(
	c: ActorContext<TState, TConnParams, TConnState, TVars, TInput, TDatabase>,
): Effect.Effect<string, never, never> => Effect.succeed(c.actorId);

export const getName = <
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider,
>(
	c: ActorContext<TState, TConnParams, TConnState, TVars, TInput, TDatabase>,
): Effect.Effect<string, never, never> => Effect.succeed(c.name);

export const getKey = <
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider,
>(
	c: ActorContext<TState, TConnParams, TConnState, TVars, TInput, TDatabase>,
) => Effect.succeed(c.key);

export const getRegion = <
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider,
>(
	c: ActorContext<TState, TConnParams, TConnState, TVars, TInput, TDatabase>,
): Effect.Effect<string, never, never> => Effect.succeed(c.region);

export const getSchedule = <
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider,
>(
	c: ActorContext<TState, TConnParams, TConnState, TVars, TInput, TDatabase>,
) => Effect.succeed(c.schedule);

export const getConns = <
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider,
>(
	c: ActorContext<TState, TConnParams, TConnState, TVars, TInput, TDatabase>,
) => Effect.succeed(c.conns);

export const getClient = <
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider,
	R extends Registry<any>,
>(
	c: ActorContext<TState, TConnParams, TConnState, TVars, TInput, TDatabase>,
) => Effect.succeed(c.client<R>());

export const getDb = <
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider,
>(
	c: ActorContext<TState, TConnParams, TConnState, TVars, TInput, TDatabase>,
) => Effect.succeed(c.db);

export const saveState = <
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider,
>(
	c: ActorContext<TState, TConnParams, TConnState, TVars, TInput, TDatabase>,
	opts: Parameters<typeof c.saveState>[0],
): Effect.Effect<void, never, never> => Effect.promise(() => c.saveState(opts));

export const waitUntil = <
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider,
	A,
	E,
>(
	c: ActorContext<TState, TConnParams, TConnState, TVars, TInput, TDatabase>,
	effect: Effect.Effect<A, E, never>,
): Effect.Effect<void, never, never> =>
	Effect.sync(() => {
		const promise = Effect.runPromise(effect).then(() => {});
		c.waitUntil(promise);
	});

export const getAbortSignal = <
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider,
>(
	c: ActorContext<TState, TConnParams, TConnState, TVars, TInput, TDatabase>,
): Effect.Effect<AbortSignal, never, never> => Effect.succeed(c.abortSignal);

export const sleep = <
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider,
>(
	c: ActorContext<TState, TConnParams, TConnState, TVars, TInput, TDatabase>,
): Effect.Effect<void, never, never> => Effect.sync(() => c.sleep());

export const destroy = <
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider,
>(
	c: ActorContext<TState, TConnParams, TConnState, TVars, TInput, TDatabase>,
): Effect.Effect<void, never, never> => Effect.sync(() => c.destroy());

export function effect<
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider,
	AEff,
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
	) => Generator<YieldWrap<Effect.Effect<any, any, any>>, AEff, never>,
): (
	c: ActorContext<TState, TConnParams, TConnState, TVars, TInput, TDatabase>,
) => Promise<AEff> {
	return (c) => {
		const gen = genFn(c);
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

export function workflow<
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider,
	AEff,
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
	) => Generator<YieldWrap<Effect.Effect<any, any, any>>, AEff, never>,
): (
	c: ActorContext<TState, TConnParams, TConnState, TVars, TInput, TDatabase>,
) => Promise<AEff> {
	return (c) => {
		const gen = genFn(c);
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
