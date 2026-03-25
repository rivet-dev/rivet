import { Cause, Context, Effect, Exit } from "effect";
import type { ActorContext } from "rivetkit";
import type { YieldWrap } from "effect/Utils";
import { StatePersistenceError } from "./errors.ts";
import { runPromise, runPromiseExit } from "./runtime.ts";
import type { AnyDatabaseProvider } from "./rivet-actor.ts";

type AnyActorContext = ActorContext<any, any, any, any, any, any>;

/**
 * Context.Tag for injecting Rivet's ActorContext into Effect pipelines.
 *
 * Uses Context.Tag (not Effect.Service) because the actor context is an
 * externally-provided runtime resource injected by the Rivet framework,
 * not a service we construct via layers.
 */
export class RivetActorContext extends Context.Tag("@rivetkit/effect/RivetActorContext")<
	RivetActorContext,
	AnyActorContext
>() {}

export const provideActorContext = <A, E, R>(
	effect: Effect.Effect<A, E, R>,
	context: unknown,
): Effect.Effect<A, E, Exclude<R, RivetActorContext>> =>
	Effect.provideService(
		effect as Effect.Effect<A, E, R | RivetActorContext>,
		RivetActorContext,
		context as AnyActorContext,
	) as Effect.Effect<A, E, Exclude<R, RivetActorContext>>;

export const context = <TState, TConnParams, TConnState, TVars, TInput, TDatabase extends AnyDatabaseProvider = AnyDatabaseProvider>(): Effect.Effect<
	ActorContext<TState, TConnParams, TConnState, TVars, TInput, TDatabase>,
	never,
	RivetActorContext
> => RivetActorContext as any;

export const state = <TState, TConnParams, TConnState, TVars, TInput, TDatabase extends AnyDatabaseProvider = AnyDatabaseProvider>(
	c: ActorContext<TState, TConnParams, TConnState, TVars, TInput, TDatabase>,
): Effect.Effect<TState, never, never> => Effect.succeed(c.state);

export const updateState = <TState, TConnParams, TConnState, TVars, TInput, TDatabase extends AnyDatabaseProvider = AnyDatabaseProvider>(
	c: ActorContext<TState, TConnParams, TConnState, TVars, TInput, TDatabase>,
	f: (state: TState) => void,
): Effect.Effect<void, never, never> => Effect.sync(() => f(c.state));

export const vars = <TState, TConnParams, TConnState, TVars, TInput, TDatabase extends AnyDatabaseProvider = AnyDatabaseProvider>(
	c: ActorContext<TState, TConnParams, TConnState, TVars, TInput, TDatabase>,
): Effect.Effect<TVars, never, never> => Effect.succeed(c.vars);

export const updateVars = <TState, TConnParams, TConnState, TVars, TInput, TDatabase extends AnyDatabaseProvider = AnyDatabaseProvider>(
	c: ActorContext<TState, TConnParams, TConnState, TVars, TInput, TDatabase>,
	f: (vars: TVars) => void,
): Effect.Effect<void, never, never> => Effect.sync(() => f(c.vars));

export const broadcast = <
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider = AnyDatabaseProvider,
	Args extends Array<unknown> = unknown[],
>(
	c: ActorContext<TState, TConnParams, TConnState, TVars, TInput, TDatabase>,
	name: string,
	...args: Args
): Effect.Effect<void, never, never> => Effect.sync(() => c.broadcast(name, ...args));

export const getLog = <TState, TConnParams, TConnState, TVars, TInput, TDatabase extends AnyDatabaseProvider = AnyDatabaseProvider>(
	c: ActorContext<TState, TConnParams, TConnState, TVars, TInput, TDatabase>,
) => Effect.succeed(c.log);

export const getActorId = <TState, TConnParams, TConnState, TVars, TInput, TDatabase extends AnyDatabaseProvider = AnyDatabaseProvider>(
	c: ActorContext<TState, TConnParams, TConnState, TVars, TInput, TDatabase>,
) => Effect.succeed(c.actorId);

export const getName = <TState, TConnParams, TConnState, TVars, TInput, TDatabase extends AnyDatabaseProvider = AnyDatabaseProvider>(
	c: ActorContext<TState, TConnParams, TConnState, TVars, TInput, TDatabase>,
) => Effect.succeed(c.name);

export const getKey = <TState, TConnParams, TConnState, TVars, TInput, TDatabase extends AnyDatabaseProvider = AnyDatabaseProvider>(
	c: ActorContext<TState, TConnParams, TConnState, TVars, TInput, TDatabase>,
) => Effect.succeed(c.key);

export const getRegion = <TState, TConnParams, TConnState, TVars, TInput, TDatabase extends AnyDatabaseProvider = AnyDatabaseProvider>(
	c: ActorContext<TState, TConnParams, TConnState, TVars, TInput, TDatabase>,
) => Effect.succeed(c.region);

export const getSchedule = <TState, TConnParams, TConnState, TVars, TInput, TDatabase extends AnyDatabaseProvider = AnyDatabaseProvider>(
	c: ActorContext<TState, TConnParams, TConnState, TVars, TInput, TDatabase>,
) => Effect.succeed(c.schedule);

export const getConns = <TState, TConnParams, TConnState, TVars, TInput, TDatabase extends AnyDatabaseProvider = AnyDatabaseProvider>(
	c: ActorContext<TState, TConnParams, TConnState, TVars, TInput, TDatabase>,
) => Effect.succeed(c.conns);

export const getClient = <TState, TConnParams, TConnState, TVars, TInput, TDatabase extends AnyDatabaseProvider = AnyDatabaseProvider>(
	c: ActorContext<TState, TConnParams, TConnState, TVars, TInput, TDatabase>,
) => Effect.succeed(c.client());

export const getDb = <TState, TConnParams, TConnState, TVars, TInput, TDatabase extends AnyDatabaseProvider = AnyDatabaseProvider>(
	c: ActorContext<TState, TConnParams, TConnState, TVars, TInput, TDatabase>,
) => Effect.succeed(c.db);

export const getKv = <TState, TConnParams, TConnState, TVars, TInput, TDatabase extends AnyDatabaseProvider = AnyDatabaseProvider>(
	c: ActorContext<TState, TConnParams, TConnState, TVars, TInput, TDatabase>,
) => Effect.succeed(c.kv);

export const getQueue = <TState, TConnParams, TConnState, TVars, TInput, TDatabase extends AnyDatabaseProvider = AnyDatabaseProvider>(
	c: ActorContext<TState, TConnParams, TConnState, TVars, TInput, TDatabase>,
) => Effect.succeed((c as unknown as { queue: unknown }).queue);

export const saveState = <TState, TConnParams, TConnState, TVars, TInput, TDatabase extends AnyDatabaseProvider = AnyDatabaseProvider>(
	c: ActorContext<TState, TConnParams, TConnState, TVars, TInput, TDatabase>,
	opts: Parameters<typeof c.saveState>[0],
): Effect.Effect<void, StatePersistenceError, never> =>
	Effect.tryPromise({
		try: () => c.saveState(opts),
		catch: (error) =>
			new StatePersistenceError({
				message: "Failed to persist actor state",
				cause: error,
			}),
	});

const logRuntimeFailure = (context: unknown, message: string, error: unknown): void => {
	if (typeof context !== "object" || context === null) return;
	const ctx = context as { log?: { error: (entry: Record<string, unknown>) => void } };
	ctx.log?.error({
		msg: message,
		error:
			error instanceof Error
				? error.message
				: typeof error === "string"
					? error
					: JSON.stringify(error),
	});
};

const runEffectOnActorContext = <A, E, R>(c: unknown, effect: Effect.Effect<A, E, R>): Promise<A> => {
	const withContext = provideActorContext(effect, c);
	return runPromise(withContext, c).catch((error) => {
		logRuntimeFailure(c, "actor effect failed", error);
		return Promise.reject(error);
	});
};

export const waitUntil = <TState, TConnParams, TConnState, TVars, TInput, TDatabase extends AnyDatabaseProvider = AnyDatabaseProvider, A = any, E = any, R = never>(
	c: ActorContext<TState, TConnParams, TConnState, TVars, TInput, TDatabase>,
	effect: Effect.Effect<A, E, R>,
): Effect.Effect<void, never, never> =>
	Effect.sync(() => {
		const promise = runPromiseExit(effect, c).then((exit) => {
			if (Exit.isFailure(exit)) {
				c.log.error({
					msg: "waitUntil effect failed",
					cause: Cause.pretty(exit.cause),
				});
			}
		});
		c.waitUntil(promise);
	});

export const getAbortSignal = <TState, TConnParams, TConnState, TVars, TInput, TDatabase extends AnyDatabaseProvider = AnyDatabaseProvider>(
	c: ActorContext<TState, TConnParams, TConnState, TVars, TInput, TDatabase>,
): Effect.Effect<AbortSignal, never, never> => Effect.succeed(c.abortSignal);

export const sleep = <TState, TConnParams, TConnState, TVars, TInput, TDatabase extends AnyDatabaseProvider = AnyDatabaseProvider>(
	c: ActorContext<TState, TConnParams, TConnState, TVars, TInput, TDatabase>,
): Effect.Effect<void, never, never> => Effect.sync(() => c.sleep());

export const destroy = <TState, TConnParams, TConnState, TVars, TInput, TDatabase extends AnyDatabaseProvider = AnyDatabaseProvider>(
	c: ActorContext<TState, TConnParams, TConnState, TVars, TInput, TDatabase>,
): Effect.Effect<void, never, never> => Effect.sync(() => c.destroy());

/** Wraps a generator function as a Rivet hook handler with Effect support. Errors propagate normally. */
export function effect<TState, TConnParams, TConnState, TVars, TInput, TDatabase extends AnyDatabaseProvider = AnyDatabaseProvider, AEff = void>(
	genFn: (
		c: ActorContext<TState, TConnParams, TConnState, TVars, TInput, TDatabase>,
	) => Generator<YieldWrap<Effect.Effect<any, any, any>>, AEff, never>,
): (c: ActorContext<TState, TConnParams, TConnState, TVars, TInput, TDatabase>) => Promise<AEff> {
	return (c) => {
		const eff = Effect.gen<YieldWrap<Effect.Effect<any, any, any>>, AEff>(() => genFn(c));
		return runEffectOnActorContext(c, eff);
	};
}
