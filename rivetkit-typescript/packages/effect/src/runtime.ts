import { Cause, Effect, Exit, Layer, ManagedRuntime, Option } from "effect";
import { RuntimeExecutionError } from "./errors.ts";

export type AnyManagedRuntime = ManagedRuntime.ManagedRuntime<any, any>;

// Exported for backwards compatibility; no longer used internally.
export const ActorManagedRuntimeSymbol = Symbol.for("@rivetkit/effect/runtime");

const DefaultRuntime = ManagedRuntime.make(Layer.empty);

const RuntimeContextMap = new WeakMap<object, AnyManagedRuntime>();

export const setManagedRuntime = (context: unknown, runtime: AnyManagedRuntime): void => {
	if (typeof context === "object" && context !== null) {
		RuntimeContextMap.set(context, runtime);
	}
};

export const getManagedRuntime = (context: unknown): AnyManagedRuntime | undefined => {
	if (typeof context === "object" && context !== null) {
		return RuntimeContextMap.get(context);
	}
};

/**
 * Last-resort fallback: runs an effect using the empty DefaultRuntime.
 * Only works when `R = never` (no unsatisfied requirements).
 */
const runWithCurrentRuntime = <A, E>(effect: Effect.Effect<A, E, never>): Promise<A> =>
	DefaultRuntime.runPromise(effect).catch((error) =>
		Promise.reject(
			new RuntimeExecutionError({
				message: "Failed to execute effect with current runtime",
				operation: "runWithCurrentRuntime",
				cause: error,
			}),
		),
	);

/**
 * Last-resort fallback: runs an effect to Exit using the empty DefaultRuntime.
 * Only works when `R = never` (no unsatisfied requirements).
 */
const runExitWithCurrentRuntime = <A, E>(effect: Effect.Effect<A, E, never>): Promise<Exit.Exit<A, E>> =>
	DefaultRuntime.runPromiseExit(effect).catch((error) =>
		Promise.reject(
			new RuntimeExecutionError({
				message: "Failed to execute effect exit with current runtime",
				operation: "runExitWithCurrentRuntime",
				cause: error,
			}),
		),
	);

const throwFromCause = <E>(cause: Cause.Cause<E>): never => {
	const failure = Cause.failureOption(cause);
	if (Option.isSome(failure)) {
		throw failure.value;
	}

	const defect = Cause.dieOption(cause);
	if (Option.isSome(defect)) {
		throw defect.value instanceof Error ? defect.value : new Error(String(defect.value));
	}

	throw new Error(Cause.pretty(cause));
};

export const runPromise = <A, E, R>(effect: Effect.Effect<A, E, R>, context?: unknown): Promise<A> =>
	runPromiseExit(effect, context).then((exit) => {
		if (Exit.isSuccess(exit)) {
			return exit.value;
		}
		return throwFromCause(exit.cause);
	});

export const runPromiseExit = <A, E, R>(
	effect: Effect.Effect<A, E, R>,
	context?: unknown,
): Promise<Exit.Exit<A, E>> => {
	const runtime = getManagedRuntime(context);
	const execution: Promise<Exit.Exit<A, E>> = runtime
		? runtime.runPromiseExit(effect as Effect.Effect<A, E, never>)
		: runExitWithCurrentRuntime(effect as Effect.Effect<A, E, never>);

	return execution.catch((error) =>
		Exit.die(
			new RuntimeExecutionError({
				message: "Runtime execution failed unexpectedly",
				operation: "runPromiseExit",
				cause: error,
			}),
		),
	);
};
