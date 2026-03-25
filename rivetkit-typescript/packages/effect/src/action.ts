import { Effect } from "effect";
import type { ActorContext, ActionContext } from "rivetkit";
import type { YieldWrap } from "effect/Utils";
import { provideActorContext } from "./actor.ts";
import { runPromise } from "./runtime.ts";
import type { AnyDatabaseProvider } from "./rivet-actor.ts";

export {
	state,
	updateState,
	vars,
	updateVars,
	broadcast,
	getLog,
	getActorId,
	getName,
	getKey,
	getRegion,
	getSchedule,
	getConns,
	getClient,
	getDb,
	getKv,
	getQueue,
	saveState,
	waitUntil,
	getAbortSignal,
	sleep,
	destroy,
	context,
	RivetActorContext,
} from "./actor.ts";

export const getConn = <
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider = AnyDatabaseProvider,
>(
	c: ActionContext<TState, TConnParams, TConnState, TVars, TInput, TDatabase>,
) => Effect.succeed(c.conn);

/** Wraps a generator function as a Rivet action handler with Effect support. Errors propagate normally. */
export function effect<
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider = AnyDatabaseProvider,
	AEff = void,
	Args extends unknown[] = [],
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
		const eff = Effect.gen<YieldWrap<Effect.Effect<any, any, any>>, AEff>(
			() => genFn(c, ...args),
		);
		const withContext = provideActorContext(eff, c);
		return runPromise(withContext, c);
	};
}
