import { Effect } from "effect";
import type {
	CreateContext,
	WakeContext,
	DestroyContext,
	SleepContext,
	StateChangeContext,
	BeforeConnectContext,
	ConnectContext,
	DisconnectContext,
	CreateConnStateContext,
	BeforeActionResponseContext,
	CreateVarsContext,
	RequestContext,
	WebSocketContext,
	Conn,
	UniversalWebSocket,
} from "rivetkit";
import type { YieldWrap } from "effect/Utils";
import { ActorContextTag } from "./actor.ts";

// Pattern: Each namespace exports an `effect()` function that:
// 1. Takes a generator function with the appropriate context
// 2. Returns a function that RivetKit can call
// 3. Runs the Effect and returns a Promise (or void for sync hooks)

export namespace OnCreate {
	export function effect<TState, TInput, AEff = void>(
		genFn: (
			c: CreateContext<TState, TInput, undefined>,
			input: TInput,
		) => Generator<YieldWrap<Effect.Effect<any, any, any>>, AEff, never>,
	): (
		c: CreateContext<TState, TInput, undefined>,
		input: TInput,
	) => Promise<AEff> {
		return (c, input) => {
			const gen = genFn(c, input);
			const eff = Effect.gen<YieldWrap<Effect.Effect<any, any, any>>, AEff>(
				() => gen,
			);

			const withContext = Effect.provideService(
				eff,
				ActorContextTag,
				c as any,
			) as Effect.Effect<AEff, any, never>;

			return Effect.runPromise(withContext);
		};
	}
}

export namespace OnWake {
	export function effect<
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		AEff = void,
	>(
		genFn: (
			c: WakeContext<
				TState,
				TConnParams,
				TConnState,
				TVars,
				TInput,
				undefined
			>,
		) => Generator<YieldWrap<Effect.Effect<any, any, any>>, AEff, never>,
	): (
		c: WakeContext<TState, TConnParams, TConnState, TVars, TInput, undefined>,
	) => Promise<AEff> {
		return (c) => {
			const gen = genFn(c);
			const eff = Effect.gen<YieldWrap<Effect.Effect<any, any, any>>, AEff>(
				() => gen,
			);

			const withContext = Effect.provideService(
				eff,
				ActorContextTag,
				c as any,
			) as Effect.Effect<AEff, any, never>;

			return Effect.runPromise(withContext);
		};
	}
}

export namespace Run {
	export function effect<
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		AEff = void,
	>(
		genFn: (
			c: WakeContext<
				TState,
				TConnParams,
				TConnState,
				TVars,
				TInput,
				undefined
			>,
		) => Generator<YieldWrap<Effect.Effect<any, any, any>>, AEff, never>,
	): (
		c: WakeContext<TState, TConnParams, TConnState, TVars, TInput, undefined>,
	) => Promise<AEff> {
		return (c) => {
			const gen = genFn(c);
			const eff = Effect.gen<YieldWrap<Effect.Effect<any, any, any>>, AEff>(
				() => gen,
			);

			const withContext = Effect.provideService(
				eff,
				ActorContextTag,
				c as any,
			) as Effect.Effect<AEff, any, never>;

			return Effect.runPromise(withContext);
		};
	}
}

export namespace OnDestroy {
	export function effect<
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		AEff = void,
	>(
		genFn: (
			c: DestroyContext<
				TState,
				TConnParams,
				TConnState,
				TVars,
				TInput,
				undefined
			>,
		) => Generator<YieldWrap<Effect.Effect<any, any, any>>, AEff, never>,
	): (
		c: DestroyContext<
			TState,
			TConnParams,
			TConnState,
			TVars,
			TInput,
			undefined
		>,
	) => Promise<AEff> {
		return (c) => {
			const gen = genFn(c);
			const eff = Effect.gen<YieldWrap<Effect.Effect<any, any, any>>, AEff>(
				() => gen,
			);

			const withContext = Effect.provideService(
				eff,
				ActorContextTag,
				c as any,
			) as Effect.Effect<AEff, any, never>;

			return Effect.runPromise(withContext);
		};
	}
}

export namespace OnSleep {
	export function effect<
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		AEff = void,
	>(
		genFn: (
			c: SleepContext<
				TState,
				TConnParams,
				TConnState,
				TVars,
				TInput,
				undefined
			>,
		) => Generator<YieldWrap<Effect.Effect<any, any, any>>, AEff, never>,
	): (
		c: SleepContext<TState, TConnParams, TConnState, TVars, TInput, undefined>,
	) => Promise<AEff> {
		return (c) => {
			const gen = genFn(c);
			const eff = Effect.gen<YieldWrap<Effect.Effect<any, any, any>>, AEff>(
				() => gen,
			);

			const withContext = Effect.provideService(
				eff,
				ActorContextTag,
				c as any,
			) as Effect.Effect<AEff, any, never>;

			return Effect.runPromise(withContext);
		};
	}
}

// OnStateChange is synchronous - uses Effect.runSync instead of Effect.runPromise
export namespace OnStateChange {
	export function effect<
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
	>(
		genFn: (
			c: StateChangeContext<
				TState,
				TConnParams,
				TConnState,
				TVars,
				TInput,
				undefined
			>,
			newState: TState,
		) => Generator<YieldWrap<Effect.Effect<any, never, any>>, void, never>,
	): (
		c: StateChangeContext<
			TState,
			TConnParams,
			TConnState,
			TVars,
			TInput,
			undefined
		>,
		newState: TState,
	) => void {
		return (c, newState) => {
			const gen = genFn(c, newState);
			const eff = Effect.gen<YieldWrap<Effect.Effect<any, never, any>>, void>(
				() => gen,
			);

			const withContext = Effect.provideService(
				eff,
				ActorContextTag,
				c as any,
			) as Effect.Effect<void, never, never>;

			Effect.runSync(withContext);
		};
	}
}

export namespace OnBeforeConnect {
	export function effect<
		TState,
		TConnParams,
		TVars,
		TInput,
		AEff = void,
	>(
		genFn: (
			c: BeforeConnectContext<TState, TVars, TInput, undefined>,
			params: TConnParams,
		) => Generator<YieldWrap<Effect.Effect<any, any, any>>, AEff, never>,
	): (
		c: BeforeConnectContext<TState, TVars, TInput, undefined>,
		params: TConnParams,
	) => Promise<AEff> {
		return (c, params) => {
			const gen = genFn(c, params);
			const eff = Effect.gen<YieldWrap<Effect.Effect<any, any, any>>, AEff>(
				() => gen,
			);

			const withContext = Effect.provideService(
				eff,
				ActorContextTag,
				c as any,
			) as Effect.Effect<AEff, any, never>;

			return Effect.runPromise(withContext);
		};
	}
}

export namespace OnConnect {
	export function effect<
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		AEff = void,
	>(
		genFn: (
			c: ConnectContext<
				TState,
				TConnParams,
				TConnState,
				TVars,
				TInput,
				undefined
			>,
			conn: Conn<TState, TConnParams, TConnState, TVars, TInput, undefined>,
		) => Generator<YieldWrap<Effect.Effect<any, any, any>>, AEff, never>,
	): (
		c: ConnectContext<
			TState,
			TConnParams,
			TConnState,
			TVars,
			TInput,
			undefined
		>,
		conn: Conn<TState, TConnParams, TConnState, TVars, TInput, undefined>,
	) => Promise<AEff> {
		return (c, conn) => {
			const gen = genFn(c, conn);
			const eff = Effect.gen<YieldWrap<Effect.Effect<any, any, any>>, AEff>(
				() => gen,
			);

			const withContext = Effect.provideService(
				eff,
				ActorContextTag,
				c as any,
			) as Effect.Effect<AEff, any, never>;

			return Effect.runPromise(withContext);
		};
	}
}

export namespace OnDisconnect {
	export function effect<
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		AEff = void,
	>(
		genFn: (
			c: DisconnectContext<
				TState,
				TConnParams,
				TConnState,
				TVars,
				TInput,
				undefined
			>,
			conn: Conn<TState, TConnParams, TConnState, TVars, TInput, undefined>,
		) => Generator<YieldWrap<Effect.Effect<any, any, any>>, AEff, never>,
	): (
		c: DisconnectContext<
			TState,
			TConnParams,
			TConnState,
			TVars,
			TInput,
			undefined
		>,
		conn: Conn<TState, TConnParams, TConnState, TVars, TInput, undefined>,
	) => Promise<AEff> {
		return (c, conn) => {
			const gen = genFn(c, conn);
			const eff = Effect.gen<YieldWrap<Effect.Effect<any, any, any>>, AEff>(
				() => gen,
			);

			const withContext = Effect.provideService(
				eff,
				ActorContextTag,
				c as any,
			) as Effect.Effect<AEff, any, never>;

			return Effect.runPromise(withContext);
		};
	}
}

export namespace CreateConnState {
	export function effect<
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
	>(
		genFn: (
			c: CreateConnStateContext<TState, TVars, TInput, undefined>,
			params: TConnParams,
		) => Generator<
			YieldWrap<Effect.Effect<any, any, any>>,
			TConnState,
			never
		>,
	): (
		c: CreateConnStateContext<TState, TVars, TInput, undefined>,
		params: TConnParams,
	) => Promise<TConnState> {
		return (c, params) => {
			const gen = genFn(c, params);
			const eff = Effect.gen<
				YieldWrap<Effect.Effect<any, any, any>>,
				TConnState
			>(() => gen);

			const withContext = Effect.provideService(
				eff,
				ActorContextTag,
				c as any,
			) as Effect.Effect<TConnState, any, never>;

			return Effect.runPromise(withContext);
		};
	}
}

export namespace OnBeforeActionResponse {
	export function effect<
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		Out = any,
	>(
		genFn: (
			c: BeforeActionResponseContext<
				TState,
				TConnParams,
				TConnState,
				TVars,
				TInput,
				undefined
			>,
			name: string,
			args: unknown[],
			output: Out,
		) => Generator<YieldWrap<Effect.Effect<any, any, any>>, Out, never>,
	): (
		c: BeforeActionResponseContext<
			TState,
			TConnParams,
			TConnState,
			TVars,
			TInput,
			undefined
		>,
		name: string,
		args: unknown[],
		output: Out,
	) => Promise<Out> {
		return (c, name, args, output) => {
			const gen = genFn(c, name, args, output);
			const eff = Effect.gen<YieldWrap<Effect.Effect<any, any, any>>, Out>(
				() => gen,
			);

			const withContext = Effect.provideService(
				eff,
				ActorContextTag,
				c as any,
			) as Effect.Effect<Out, any, never>;

			return Effect.runPromise(withContext);
		};
	}
}

export namespace CreateState {
	export function effect<TState, TInput>(
		genFn: (
			c: CreateContext<TState, TInput, undefined>,
			input: TInput,
		) => Generator<YieldWrap<Effect.Effect<any, any, any>>, TState, never>,
	): (
		c: CreateContext<TState, TInput, undefined>,
		input: TInput,
	) => Promise<TState> {
		return (c, input) => {
			const gen = genFn(c, input);
			const eff = Effect.gen<YieldWrap<Effect.Effect<any, any, any>>, TState>(
				() => gen,
			);

			const withContext = Effect.provideService(
				eff,
				ActorContextTag,
				c as any,
			) as Effect.Effect<TState, any, never>;

			return Effect.runPromise(withContext);
		};
	}
}

export namespace CreateVars {
	export function effect<TState, TVars, TInput>(
		genFn: (
			c: CreateVarsContext<TState, TInput, undefined>,
			driverCtx: any,
		) => Generator<YieldWrap<Effect.Effect<any, any, any>>, TVars, never>,
	): (
		c: CreateVarsContext<TState, TInput, undefined>,
		driverCtx: any,
	) => Promise<TVars> {
		return (c, driverCtx) => {
			const gen = genFn(c, driverCtx);
			const eff = Effect.gen<YieldWrap<Effect.Effect<any, any, any>>, TVars>(
				() => gen,
			);

			const withContext = Effect.provideService(
				eff,
				ActorContextTag,
				c as any,
			) as Effect.Effect<TVars, any, never>;

			return Effect.runPromise(withContext);
		};
	}
}

export namespace OnRequest {
	export function effect<
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
	>(
		genFn: (
			c: RequestContext<
				TState,
				TConnParams,
				TConnState,
				TVars,
				TInput,
				undefined
			>,
			request: Request,
		) => Generator<YieldWrap<Effect.Effect<any, any, any>>, Response, never>,
	): (
		c: RequestContext<
			TState,
			TConnParams,
			TConnState,
			TVars,
			TInput,
			undefined
		>,
		request: Request,
	) => Promise<Response> {
		return (c, request) => {
			const gen = genFn(c, request);
			const eff = Effect.gen<YieldWrap<Effect.Effect<any, any, any>>, Response>(
				() => gen,
			);

			const withContext = Effect.provideService(
				eff,
				ActorContextTag,
				c as any,
			) as Effect.Effect<Response, any, never>;

			return Effect.runPromise(withContext);
		};
	}
}

export namespace OnWebSocket {
	export function effect<
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		AEff = void,
	>(
		genFn: (
			c: WebSocketContext<
				TState,
				TConnParams,
				TConnState,
				TVars,
				TInput,
				undefined
			>,
			websocket: UniversalWebSocket,
		) => Generator<YieldWrap<Effect.Effect<any, any, any>>, AEff, never>,
	): (
		c: WebSocketContext<
			TState,
			TConnParams,
			TConnState,
			TVars,
			TInput,
			undefined
		>,
		websocket: UniversalWebSocket,
	) => Promise<AEff> {
		return (c, websocket) => {
			const gen = genFn(c, websocket);
			const eff = Effect.gen<YieldWrap<Effect.Effect<any, any, any>>, AEff>(
				() => gen,
			);

			const withContext = Effect.provideService(
				eff,
				ActorContextTag,
				c as any,
			) as Effect.Effect<AEff, any, never>;

			return Effect.runPromise(withContext);
		};
	}
}
