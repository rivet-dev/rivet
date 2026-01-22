import { Effect } from "effect";
import type { ActorContext, ActionContext } from "rivetkit";
import type { YieldWrap } from "effect/Utils";
import { ActorContextTag } from "./actor.ts";

export * from "./actor.ts";

export const getConn = <
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
>(
	c: ActionContext<TState, TConnParams, TConnState, TVars, TInput, undefined>,
) => Effect.succeed(c.conn);

export function effect<
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
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
			undefined
		>,
		...args: Args
	) => Generator<YieldWrap<Effect.Effect<any, any, any>>, AEff, never>,
): (
	c: ActionContext<TState, TConnParams, TConnState, TVars, TInput, undefined>,
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

		return Effect.runPromise(withContext);
	};
}

