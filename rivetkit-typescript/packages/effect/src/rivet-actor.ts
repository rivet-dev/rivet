import type { ManagedRuntime } from "effect";
import { actor as rivetActor, type Actions, type ActorConfigInput, type ActorDefinition } from "rivetkit";
import { setManagedRuntime } from "./runtime.ts";

type ActorRuntime = ManagedRuntime.ManagedRuntime<any, any>;
export type AnyDatabaseProvider =
	Parameters<typeof rivetActor>[0] extends ActorConfigInput<any, any, any, any, any, infer TDatabase, any, any, any>
		? TDatabase
		: never;

const withContextRuntime = (
	fn: (...args: unknown[]) => unknown,
	runtime: ActorRuntime,
): ((...args: unknown[]) => unknown) => {
	return (...args: unknown[]) => {
		setManagedRuntime(args[0], runtime);
		return fn(...args);
	};
};

const wrapActorConfigWithRuntime = (
	input: Record<string, unknown>,
	runtime: ActorRuntime,
): Record<string, unknown> => {
	const wrapped = { ...input };

	const hookKeys = [
		"onCreate",
		"onWake",
		"onDestroy",
		"onSleep",
		"onStateChange",
		"onBeforeConnect",
		"onConnect",
		"onDisconnect",
		"createConnState",
		"onBeforeActionResponse",
		"createState",
		"createVars",
		"onRequest",
		"onWebSocket",
	] as const;

	for (const hookKey of hookKeys) {
		const hook = wrapped[hookKey];
		if (typeof hook === "function") {
			wrapped[hookKey] = withContextRuntime(hook as (...args: unknown[]) => unknown, runtime);
		}
	}

	const actions = wrapped.actions;
	if (actions && typeof actions === "object") {
		const wrappedActions: Record<string, unknown> = {};
		for (const [name, action] of Object.entries(actions)) {
			wrappedActions[name] =
				typeof action === "function"
					? withContextRuntime(action as (...args: unknown[]) => unknown, runtime)
					: action;
		}
		wrapped.actions = wrappedActions;
	}

	return wrapped;
};

export function actor<
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider,
	TActions extends Actions<TState, TConnParams, TConnState, TVars, TInput, TDatabase>,
>(
	input: ActorConfigInput<
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		TDatabase,
		Record<never, never>,
		Record<never, never>,
		TActions
	> & {
		runtime?: ActorRuntime;
	},
): ActorDefinition<
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase,
	Record<never, never>,
	Record<never, never>,
	TActions
> {
	const { runtime, ...config } = input;
	const runtimeAwareConfig = runtime
		? wrapActorConfigWithRuntime(config as Record<string, unknown>, runtime)
		: config;

	return rivetActor(
		runtimeAwareConfig as ActorConfigInput<
			TState,
			TConnParams,
			TConnState,
			TVars,
			TInput,
			TDatabase,
			Record<never, never>,
			Record<never, never>,
			TActions
		>,
	) as ActorDefinition<
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		TDatabase,
		Record<never, never>,
		Record<never, never>,
		TActions
	>;
}
