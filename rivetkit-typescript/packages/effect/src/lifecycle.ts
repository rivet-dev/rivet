import { Cause, Effect, Exit } from "effect";
import type { AnyDatabaseProvider } from "./rivet-actor.ts";
import type {
	BeforeActionResponseContext,
	BeforeConnectContext,
	ConnectContext,
	Conn,
	CreateConnStateContext,
	CreateContext,
	CreateVarsContext,
	DestroyContext,
	DisconnectContext,
	RequestContext,
	RunContext,
	SleepContext,
	StateChangeContext,
	UniversalWebSocket,
	WakeContext,
	WebSocketContext,
} from "rivetkit";
import type { YieldWrap } from "effect/Utils";
import { provideActorContext } from "./actor.ts";
import { runPromise, runPromiseExit } from "./runtime.ts";

const runWithContext = <A, E, R>(
	context: unknown,
	effect: Effect.Effect<A, E, R>,
): Promise<A> => runPromise(provideActorContext(effect, context), context);

const runWithContextExit = <A, E, R>(
	context: unknown,
	effect: Effect.Effect<A, E, R>,
): Promise<Exit.Exit<A, E>> =>
	runPromiseExit(provideActorContext(effect, context), context);

const makeAsyncLifecycle = <C, Args extends unknown[], AEff>(
	genFn: (
		context: C,
		...args: Args
	) => Generator<YieldWrap<Effect.Effect<any, any, any>>, AEff, never>,
) => {
	return (context: C, ...args: Args): Promise<AEff> =>
		runWithContext(
			context,
			Effect.gen(() => genFn(context, ...args)),
		);
};

export namespace OnCreate {
	export const effect = <
		TState,
		TInput,
		TDatabase extends AnyDatabaseProvider = AnyDatabaseProvider,
		AEff = void,
	>(
		genFn: (
			c: CreateContext<TState, TInput, TDatabase>,
			input: TInput,
		) => Generator<YieldWrap<Effect.Effect<any, any, any>>, AEff, never>,
	): ((
		c: CreateContext<TState, TInput, TDatabase>,
		input: TInput,
	) => Promise<AEff>) => makeAsyncLifecycle(genFn);
}

export namespace OnWake {
	export const effect = <
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		TDatabase extends AnyDatabaseProvider = AnyDatabaseProvider,
		AEff = void,
	>(
		genFn: (
			c: WakeContext<
				TState,
				TConnParams,
				TConnState,
				TVars,
				TInput,
				TDatabase
			>,
		) => Generator<YieldWrap<Effect.Effect<any, any, any>>, AEff, never>,
	): ((
		c: WakeContext<
			TState,
			TConnParams,
			TConnState,
			TVars,
			TInput,
			TDatabase
		>,
	) => Promise<AEff>) => makeAsyncLifecycle(genFn);
}

export namespace OnRun {
	export const effect = <
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		TDatabase extends AnyDatabaseProvider = AnyDatabaseProvider,
		AEff = void,
	>(
		genFn: (
			c: RunContext<
				TState,
				TConnParams,
				TConnState,
				TVars,
				TInput,
				TDatabase
			>,
		) => Generator<YieldWrap<Effect.Effect<any, any, any>>, AEff, never>,
	): ((
		c: RunContext<
			TState,
			TConnParams,
			TConnState,
			TVars,
			TInput,
			TDatabase
		>,
	) => Promise<AEff>) => makeAsyncLifecycle(genFn);
}

export namespace OnDestroy {
	export const effect = <
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		TDatabase extends AnyDatabaseProvider = AnyDatabaseProvider,
		AEff = void,
	>(
		genFn: (
			c: DestroyContext<
				TState,
				TConnParams,
				TConnState,
				TVars,
				TInput,
				TDatabase
			>,
		) => Generator<YieldWrap<Effect.Effect<any, any, any>>, AEff, never>,
	): ((
		c: DestroyContext<
			TState,
			TConnParams,
			TConnState,
			TVars,
			TInput,
			TDatabase
		>,
	) => Promise<AEff>) => makeAsyncLifecycle(genFn);
}

export namespace OnSleep {
	export const effect = <
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		TDatabase extends AnyDatabaseProvider = AnyDatabaseProvider,
		AEff = void,
	>(
		genFn: (
			c: SleepContext<
				TState,
				TConnParams,
				TConnState,
				TVars,
				TInput,
				TDatabase
			>,
		) => Generator<YieldWrap<Effect.Effect<any, any, any>>, AEff, never>,
	): ((
		c: SleepContext<
			TState,
			TConnParams,
			TConnState,
			TVars,
			TInput,
			TDatabase
		>,
	) => Promise<AEff>) => makeAsyncLifecycle(genFn);
}

export namespace OnStateChange {
	export function effect<
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		TDatabase extends AnyDatabaseProvider = AnyDatabaseProvider,
	>(
		genFn: (
			c: StateChangeContext<
				TState,
				TConnParams,
				TConnState,
				TVars,
				TInput,
				TDatabase
			>,
			newState: TState,
		) => Generator<YieldWrap<Effect.Effect<any, any, any>>, void, never>,
	): (
		c: StateChangeContext<
			TState,
			TConnParams,
			TConnState,
			TVars,
			TInput,
			TDatabase
		>,
		newState: TState,
	) => void {
		return (c, newState) => {
			void runWithContextExit(
				c,
				Effect.gen(() => genFn(c, newState)),
			)
				.then((exit) => {
					if (Exit.isFailure(exit)) {
						c.log.error({
							msg: "onStateChange effect failed",
							cause: Cause.pretty(exit.cause),
						});
					}
				})
				.catch((error) => {
					c.log.error({
						msg: "onStateChange effect threw unexpectedly",
						error: String(error),
					});
				});
		};
	}
}

export namespace OnBeforeConnect {
	export const effect = <
		TState,
		TConnParams,
		TVars,
		TInput,
		TDatabase extends AnyDatabaseProvider = AnyDatabaseProvider,
		AEff = void,
	>(
		genFn: (
			c: BeforeConnectContext<TState, TVars, TInput, TDatabase>,
			params: TConnParams,
		) => Generator<YieldWrap<Effect.Effect<any, any, any>>, AEff, never>,
	): ((
		c: BeforeConnectContext<TState, TVars, TInput, TDatabase>,
		params: TConnParams,
	) => Promise<AEff>) => makeAsyncLifecycle(genFn);
}

export namespace OnConnect {
	export const effect = <
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		TDatabase extends AnyDatabaseProvider = AnyDatabaseProvider,
		AEff = void,
	>(
		genFn: (
			c: ConnectContext<
				TState,
				TConnParams,
				TConnState,
				TVars,
				TInput,
				TDatabase
			>,
			conn: Conn<
				TState,
				TConnParams,
				TConnState,
				TVars,
				TInput,
				TDatabase
			>,
		) => Generator<YieldWrap<Effect.Effect<any, any, any>>, AEff, never>,
	): ((
		c: ConnectContext<
			TState,
			TConnParams,
			TConnState,
			TVars,
			TInput,
			TDatabase
		>,
		conn: Conn<TState, TConnParams, TConnState, TVars, TInput, TDatabase>,
	) => Promise<AEff>) => makeAsyncLifecycle(genFn);
}

export namespace OnDisconnect {
	export const effect = <
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		TDatabase extends AnyDatabaseProvider = AnyDatabaseProvider,
		AEff = void,
	>(
		genFn: (
			c: DisconnectContext<
				TState,
				TConnParams,
				TConnState,
				TVars,
				TInput,
				TDatabase
			>,
			conn: Conn<
				TState,
				TConnParams,
				TConnState,
				TVars,
				TInput,
				TDatabase
			>,
		) => Generator<YieldWrap<Effect.Effect<any, any, any>>, AEff, never>,
	): ((
		c: DisconnectContext<
			TState,
			TConnParams,
			TConnState,
			TVars,
			TInput,
			TDatabase
		>,
		conn: Conn<TState, TConnParams, TConnState, TVars, TInput, TDatabase>,
	) => Promise<AEff>) => makeAsyncLifecycle(genFn);
}

export namespace CreateConnState {
	export const effect = <
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		TDatabase extends AnyDatabaseProvider = AnyDatabaseProvider,
	>(
		genFn: (
			c: CreateConnStateContext<TState, TVars, TInput, TDatabase>,
			params: TConnParams,
		) => Generator<
			YieldWrap<Effect.Effect<any, any, any>>,
			TConnState,
			never
		>,
	): ((
		c: CreateConnStateContext<TState, TVars, TInput, TDatabase>,
		params: TConnParams,
	) => Promise<TConnState>) => makeAsyncLifecycle(genFn);
}

export namespace OnBeforeActionResponse {
	export const effect = <
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		TDatabase extends AnyDatabaseProvider = AnyDatabaseProvider,
		Out = any,
	>(
		genFn: (
			c: BeforeActionResponseContext<
				TState,
				TConnParams,
				TConnState,
				TVars,
				TInput,
				TDatabase
			>,
			name: string,
			args: unknown[],
			output: Out,
		) => Generator<YieldWrap<Effect.Effect<any, any, any>>, Out, never>,
	): ((
		c: BeforeActionResponseContext<
			TState,
			TConnParams,
			TConnState,
			TVars,
			TInput,
			TDatabase
		>,
		name: string,
		args: unknown[],
		output: Out,
	) => Promise<Out>) => makeAsyncLifecycle(genFn);
}

export namespace CreateState {
	export const effect = <
		TState,
		TInput,
		TDatabase extends AnyDatabaseProvider = AnyDatabaseProvider,
	>(
		genFn: (
			c: CreateContext<TState, TInput, TDatabase>,
			input: TInput,
		) => Generator<YieldWrap<Effect.Effect<any, any, any>>, TState, never>,
	): ((
		c: CreateContext<TState, TInput, TDatabase>,
		input: TInput,
	) => Promise<TState>) => makeAsyncLifecycle(genFn);
}

export namespace CreateVars {
	export const effect = <
		TState,
		TVars,
		TInput,
		TDatabase extends AnyDatabaseProvider = AnyDatabaseProvider,
	>(
		genFn: (
			c: CreateVarsContext<TState, TInput, TDatabase>,
			driverCtx: unknown,
		) => Generator<YieldWrap<Effect.Effect<any, any, any>>, TVars, never>,
	): ((
		c: CreateVarsContext<TState, TInput, TDatabase>,
		driverCtx: unknown,
	) => Promise<TVars>) => makeAsyncLifecycle(genFn);
}

export namespace OnRequest {
	export const effect = <
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		TDatabase extends AnyDatabaseProvider = AnyDatabaseProvider,
	>(
		genFn: (
			c: RequestContext<
				TState,
				TConnParams,
				TConnState,
				TVars,
				TInput,
				TDatabase
			>,
			request: Request,
		) => Generator<
			YieldWrap<Effect.Effect<any, any, any>>,
			Response,
			never
		>,
	): ((
		c: RequestContext<
			TState,
			TConnParams,
			TConnState,
			TVars,
			TInput,
			TDatabase
		>,
		request: Request,
	) => Promise<Response>) => makeAsyncLifecycle(genFn);
}

export namespace OnWebSocket {
	export const effect = <
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		TDatabase extends AnyDatabaseProvider = AnyDatabaseProvider,
		AEff = void,
	>(
		genFn: (
			c: WebSocketContext<
				TState,
				TConnParams,
				TConnState,
				TVars,
				TInput,
				TDatabase
			>,
			websocket: UniversalWebSocket,
		) => Generator<YieldWrap<Effect.Effect<any, any, any>>, AEff, never>,
	): ((
		c: WebSocketContext<
			TState,
			TConnParams,
			TConnState,
			TVars,
			TInput,
			TDatabase
		>,
		websocket: UniversalWebSocket,
	) => Promise<AEff>) => makeAsyncLifecycle(genFn);
}
