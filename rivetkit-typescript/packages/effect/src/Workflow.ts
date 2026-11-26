/**
 * Workflow contracts and the durable steps of a workflow. `make` defines a
 * contract. Use the other effects only in the workflow body.
 *
 * RivetKit runs the workflow body again from the start on every wake, and
 * after each durable sleep and step retry. A finished step, sleep or loop
 * iteration is not run again: it returns its saved result. So everything
 * outside a step must be deterministic. Put each side effect, and each read
 * of the clock, of random values or of SQLite, in a `step`.
 *
 * Run these effects one at a time. RivetKit rejects concurrent steps. To run
 * branches at the same time, use `join` or `race`.
 */

import type * as WorkflowsPackage from "@rivet-dev/workflows";
import { CancelledError, Loop } from "@rivet-dev/workflows";
import {
	Cause,
	Context,
	Data,
	DateTime,
	Duration,
	Effect,
	Exit,
	Schema,
} from "effect";
import { WorkflowTeardownSignal } from "./internal/WorkflowTeardownSignal.ts";

export { make } from "./internal/WorkflowContract.ts";

/**
 * The requirement of the effects in this module. Only the workflow body
 * gets it.
 */
export class Workflow extends Context.Service<
	Workflow,
	{
		/**
		 * The workflow context of `@rivet-dev/workflows`, for the features this
		 * module does not wrap.
		 */
		readonly rawRivetkitContext: RivetkitWorkflowContext;
	}
>()("@rivetkit/effect/Workflow") {}

export type RivetkitWorkflowContext = WorkflowsPackage.WorkflowContext<
	any,
	any,
	any,
	any,
	any,
	any,
	any,
	any
>;

/**
 * What `onError` in the `toLayer` options of a workflow gets: `step` for a
 * failed step attempt, `rollback` for a failed rollback and `workflow` for a
 * failure outside a step.
 */
export type ErrorEvent = WorkflowsPackage.WorkflowErrorEvent;

export type StepOptions<A = never, RollbackE = never, RollbackR = never> = {
	/** How many times a failed step runs again. Default: 3. */
	readonly maxRetries?: number;
	/** The first delay of the exponential retry backoff. Default: 100 millis. */
	readonly retryBackoffBase?: Duration.Input;
	/** The longest delay of the retry backoff. Default: 30 seconds. */
	readonly retryBackoffMax?: Duration.Input;
	/**
	 * How long one attempt can run. After that, the effect is interrupted.
	 * Default: 30 seconds. `Duration.infinity` disables it.
	 */
	readonly timeout?: Duration.Input;
	/** Retries a timed-out attempt. By default a timeout fails the workflow. */
	readonly retryOnTimeout?: boolean;
	/**
	 * Saves the result with the next write of the workflow history instead
	 * of at once, which saves a write. If the actor stops before that write,
	 * for example in a crash, the step runs again. Use it only for an
	 * idempotent effect.
	 */
	readonly ephemeral?: boolean;
	/**
	 * Undoes the step. When the workflow fails, RivetKit runs the rollbacks
	 * of the finished steps, the newest first, each with the saved result of
	 * its step. A failed rollback stops the ones before it. The workflow must
	 * run `rollbackCheckpoint` before a step that has a rollback.
	 */
	readonly rollback?: (
		output: A,
	) => Effect.Effect<void, RollbackE, RollbackR>;
};

const defaultStepTimeout = Duration.seconds(30);

/** `@rivet-dev/workflows` reads a timeout of 0 as no timeout. */
const disabledStepTimeoutMillis = 0;

/**
 * The longest delay that `setTimeout` takes. With a longer step timeout the
 * engine still ends the attempt, but the SDK does not interrupt `effect`.
 */
const maxTimerDelayMillis = 2 ** 31 - 1;

/**
 * Runs `effect` on a fiber of its own and rejects with the original error, so
 * RivetKit sees the control errors that its workflow engine throws.
 *
 * A sleep does not interrupt the fiber: RivetKit waits for the workflow
 * during the sleep grace period, so a step attempt can finish and be saved.
 * The teardown of the actor interrupts it, and the promise then rejects with
 * the engine's eviction error.
 *
 * A step attempt does not settle after the teardown or a timeout. RivetKit
 * rejects a timed-out step itself. It records any rejection of an attempt as
 * a failed attempt, so an interrupted attempt stays pending: nothing is
 * saved and the next wake runs the step again.
 */
const runPromiseWith = <R>(context: Context.Context<R>) => {
	const teardownSignal = Context.get(context, WorkflowTeardownSignal);
	return <A>(
		effect: Effect.Effect<A, unknown, R>,
		stepAttempt?: { readonly timeoutMillis: number | undefined },
	): Promise<A> =>
		new Promise((resolve, reject) => {
			const fiber = Effect.runForkWith(context)(effect, {
				signal: teardownSignal,
			});
			let timedOut = false;
			const timer =
				stepAttempt?.timeoutMillis === undefined
					? undefined
					: setTimeout(() => {
							timedOut = true;
							fiber.interruptUnsafe();
						}, stepAttempt.timeoutMillis);
			fiber.addObserver((exit) => {
				clearTimeout(timer);
				if (Exit.isSuccess(exit)) {
					resolve(exit.value);
				} else if (
					teardownSignal.aborted &&
					Cause.hasInterruptsOnly(exit.cause)
				) {
					if (stepAttempt === undefined) {
						reject(teardownSignal.reason);
					}
				} else if (!timedOut) {
					reject(Cause.squash(exit.cause));
				}
			});
		});
};

/** RivetKit's workflow engine handles eviction between its own calls. */
const callRivetkit = <A>(call: () => Promise<A>) =>
	Effect.uninterruptible(Effect.promise(call));

const provideRivetkitContext = <A, E, R>(
	effect: Effect.Effect<A, E, R>,
	rawRivetkitContext: RivetkitWorkflowContext,
): Effect.Effect<A, E, Exclude<R, Workflow>> =>
	Effect.provideService(effect, Workflow, { rawRivetkitContext });

type AnyStepOptions = StepOptions<any, unknown, any> & {
	readonly schema?: Schema.Top;
};

const makeRivetkitStep = Effect.fnUntraced(function* (
	name: string,
	effect: Effect.Effect<unknown, unknown, any>,
	options: AnyStepOptions,
) {
	const runPromise = runPromiseWith(yield* Effect.context<any>());
	const codec =
		options.schema === undefined
			? undefined
			: Schema.toCodecJson(options.schema);
	const decodeSaved = (saved: unknown): Effect.Effect<unknown, never, any> =>
		codec === undefined
			? Effect.succeed(saved)
			: Effect.orDie(Schema.decodeUnknownEffect(codec)(saved));
	const requestedTimeoutMillis = Duration.toMillis(
		options.timeout ?? defaultStepTimeout,
	);
	const timeoutMillis = Number.isFinite(requestedTimeoutMillis)
		? requestedTimeoutMillis
		: disabledStepTimeoutMillis;
	const { rollback } = options;

	return {
		decodeSaved,
		config: {
			name,
			maxRetries: options.maxRetries,
			retryBackoffBase:
				options.retryBackoffBase === undefined
					? undefined
					: Duration.toMillis(options.retryBackoffBase),
			retryBackoffMax:
				options.retryBackoffMax === undefined
					? undefined
					: Duration.toMillis(options.retryBackoffMax),
			timeout: timeoutMillis,
			retryOnTimeout: options.retryOnTimeout,
			ephemeral: options.ephemeral,
			run: () =>
				runPromise(
					codec === undefined
						? effect
						: Effect.flatMap(
								effect,
								Schema.encodeUnknownEffect(codec),
							),
					{
						timeoutMillis:
							timeoutMillis > disabledStepTimeoutMillis &&
							timeoutMillis <= maxTimerDelayMillis
								? timeoutMillis
								: undefined,
					},
				),
			rollback:
				rollback === undefined
					? undefined
					: (_: unknown, saved: unknown) =>
							runPromise(
								Effect.flatMap(decodeSaved(saved), rollback),
							),
		},
	};
});

/**
 * Runs `effect` once and saves its result. When the workflow runs again, the
 * step returns the saved result and `effect` does not run.
 *
 * A failure or defect of `effect` fails the attempt. RivetKit retries it with
 * exponential backoff, and when the retries are used up the workflow fails.
 * A crash during the attempt runs `effect` again, so make it idempotent.
 *
 * The result must be JSON-compatible. For any other type pass `schema`: the
 * result is saved with the schema's JSON codec and decoded when it is read.
 *
 * @example
 * ```ts
 * const order = yield* Workflow.step("load-order", loadOrder);
 * const paidAt = yield* Workflow.step("charge", charge(order), {
 *   schema: Schema.DateTimeUtc,
 *   maxRetries: 5,
 *   timeout: "1 minute",
 *   rollback: () => refund(order),
 * });
 * ```
 */
export const step: {
	<S extends Schema.Top, E, R, RollbackE = never, RollbackR = never>(
		name: string,
		effect: Effect.Effect<S["Type"], E, R>,
		options: StepOptions<S["Type"], RollbackE, RollbackR> & {
			readonly schema: S;
		},
	): Effect.Effect<
		S["Type"],
		never,
		| R
		| NoInfer<RollbackR>
		| Workflow
		| S["EncodingServices"]
		| S["DecodingServices"]
	>;
	<A, E, R, RollbackE = never, RollbackR = never>(
		name: string,
		effect: Effect.Effect<A, E, R>,
		options?: StepOptions<A, RollbackE, RollbackR>,
	): Effect.Effect<A, never, R | NoInfer<RollbackR> | Workflow>;
} = (
	name: string,
	effect: Effect.Effect<unknown, unknown, any>,
	options: AnyStepOptions = {},
): Effect.Effect<any, never, any> =>
	Effect.gen(function* () {
		const { rawRivetkitContext } = yield* Workflow;
		const { config, decodeSaved } = yield* makeRivetkitStep(
			name,
			effect,
			options,
		);
		return yield* decodeSaved(
			yield* callRivetkit(() => rawRivetkitContext.step(config)),
		);
	});

/** A kind of `StepFailure`, and a value of the `catch` option of `tryStep`. */
export type StepFailureKind = WorkflowsPackage.TryStepCatchKind;

/** The kinds that `tryStep` returns when its options have no `catch`. */
type DefaultStepFailureKind = "exhausted" | "timeout" | "critical";

class StepFailureError extends Data.TaggedError("WorkflowStepFailure")<{
	readonly stepName: string;
	readonly kind: StepFailureKind;
	readonly attempts: number;
	readonly message: string;
}> {}

/**
 * Why `tryStep` failed. RivetKit saves only the text of the last error of a
 * step, so the error of the effect is not available.
 */
export interface StepFailure<
	Kind extends StepFailureKind = DefaultStepFailureKind,
> extends StepFailureError {
	/**
	 * `exhausted`: the retries are used up. `timeout`: an attempt timed out.
	 * `critical`: the effect died with the `CriticalError` of
	 * `@rivet-dev/workflows`, which is not retried. `rollback`: the effect
	 * died with the `RollbackError` of `@rivet-dev/workflows`.
	 */
	readonly kind: Kind;
	/** The message of the last error, as RivetKit saved it. */
	readonly message: string;
}

export const StepFailure: new <
	Kind extends StepFailureKind = DefaultStepFailureKind,
>(args: {
	readonly stepName: string;
	readonly kind: Kind;
	readonly attempts: number;
	readonly message: string;
}) => StepFailure<Kind> = StepFailureError as any;

type TryStepOptions<
	A,
	RollbackE,
	RollbackR,
	Kind extends StepFailureKind,
> = StepOptions<A, RollbackE, RollbackR> & {
	/**
	 * The kinds of failure that `tryStep` returns as a `StepFailure`.
	 * Another failure fails the workflow, like a failed `step`. Default:
	 * `["exhausted", "timeout", "critical"]`.
	 */
	readonly catch?: ReadonlyArray<Kind>;
};

/**
 * A `step` that fails with a `StepFailure` when `step` would fail the
 * workflow. The failure is saved like a result: when the workflow runs again,
 * `tryStep` fails with it and `effect` does not run.
 *
 * When the workflow fails later, RivetKit runs no rollback for the steps
 * after a failed `tryStep`.
 *
 * @example
 * ```ts
 * const receipt = yield* Workflow.tryStep("send-receipt", sendReceipt).pipe(
 *   Effect.catchTag("WorkflowStepFailure", (failure) =>
 *     Effect.succeed(`not sent: ${failure.message}`),
 *   ),
 * );
 * ```
 */
export const tryStep: {
	<
		S extends Schema.Top,
		E,
		R,
		RollbackE = never,
		RollbackR = never,
		const Kind extends StepFailureKind = DefaultStepFailureKind,
	>(
		name: string,
		effect: Effect.Effect<S["Type"], E, R>,
		options: TryStepOptions<S["Type"], RollbackE, RollbackR, Kind> & {
			readonly schema: S;
		},
	): Effect.Effect<
		S["Type"],
		StepFailure<Kind>,
		| R
		| NoInfer<RollbackR>
		| Workflow
		| S["EncodingServices"]
		| S["DecodingServices"]
	>;
	<
		A,
		E,
		R,
		RollbackE = never,
		RollbackR = never,
		const Kind extends StepFailureKind = DefaultStepFailureKind,
	>(
		name: string,
		effect: Effect.Effect<A, E, R>,
		options?: TryStepOptions<A, RollbackE, RollbackR, Kind>,
	): Effect.Effect<A, StepFailure<Kind>, R | NoInfer<RollbackR> | Workflow>;
} = (
	name: string,
	effect: Effect.Effect<unknown, unknown, any>,
	options: AnyStepOptions & {
		readonly catch?: ReadonlyArray<StepFailureKind>;
	} = {},
): Effect.Effect<any, StepFailure<StepFailureKind>, any> =>
	Effect.gen(function* () {
		const { rawRivetkitContext } = yield* Workflow;
		const { config, decodeSaved } = yield* makeRivetkitStep(
			name,
			effect,
			options,
		);
		const result = yield* callRivetkit(() =>
			rawRivetkitContext.tryStep({ ...config, catch: options.catch }),
		);
		if (result.ok) return yield* decodeSaved(result.value);
		const { failure } = result;
		return yield* new StepFailure({
			stepName: failure.stepName,
			kind: failure.kind,
			attempts: failure.attempts,
			message: failure.error.message,
		});
	});

/** A value of the `catch` option of `try`. */
export type TryFailureKind = WorkflowsPackage.TryBlockCatchKind;

/** The kinds that `try` returns when its options have no `catch`. */
type DefaultTryFailureKind = "step" | "join" | "race";

/**
 * A failed rollback is reported by the step whose rollback failed, or by
 * the block when it died with the `RollbackError` of `@rivet-dev/workflows`.
 */
type TryFailureSource<Kind extends TryFailureKind> = Kind extends "rollback"
	? "step" | "block"
	: Kind;

class TryFailureError extends Data.TaggedError("WorkflowTryFailure")<{
	readonly source: "step" | "join" | "race" | "block";
	readonly name: string;
	readonly message: string;
}> {}

/**
 * Why `try` failed. `name` is the step, join or race that failed in the
 * block, or the block for `source: "block"`.
 */
export interface TryFailure<Kind extends TryFailureKind = DefaultTryFailureKind>
	extends TryFailureError {
	readonly source: TryFailureSource<Kind>;
	/** The message of the error, as RivetKit saved it. */
	readonly message: string;
}

export const TryFailure: new <
	Kind extends TryFailureKind = DefaultTryFailureKind,
>(args: {
	readonly source: TryFailureSource<Kind>;
	readonly name: string;
	readonly message: string;
}) => TryFailure<Kind> = TryFailureError as any;

/**
 * Runs `body` and fails with a `TryFailure` when a step, join or race in it
 * would fail the workflow. Names in `body` must be unique only within the
 * block.
 *
 * `catch` in the options sets the kinds of failure that `try` returns:
 * `step`, `join`, `race` and `rollback`. Another failure fails the
 * workflow. Default: `["step", "join", "race"]`.
 *
 * @example
 * ```ts
 * yield* Workflow.try(
 *   "notify",
 *   Effect.gen(function* () {
 *     yield* Workflow.step("send-email", sendEmail);
 *     yield* Workflow.step("send-sms", sendSms);
 *   }),
 * ).pipe(Effect.catchTag("WorkflowTryFailure", () => Effect.void));
 * ```
 */
const tryBlock = <
	A,
	R,
	const Kind extends TryFailureKind = DefaultTryFailureKind,
>(
	name: string,
	body: Effect.Effect<A, never, R>,
	options?: { readonly catch?: ReadonlyArray<Kind> },
): Effect.Effect<A, TryFailure<Kind>, R | Workflow> =>
	Effect.gen(function* () {
		const { rawRivetkitContext } = yield* Workflow;
		const runPromise = runPromiseWith(yield* Effect.context<R>());

		const result = yield* callRivetkit(() =>
			rawRivetkitContext.try({
				name,
				run: (blockContext: RivetkitWorkflowContext) =>
					runPromise(provideRivetkitContext(body, blockContext)),
				catch: options?.catch,
			}),
		);
		if (result.ok) return result.value;
		const { failure } = result;
		return yield* new TryFailure<Kind>({
			source: failure.source as TryFailureSource<Kind>,
			name: failure.name,
			message: failure.error.message,
		});
	});

export { tryBlock as try };

/**
 * Marks the point that rollbacks go back to. Run it before the first step
 * that has a `rollback`.
 */
export const rollbackCheckpoint = (
	name: string,
): Effect.Effect<void, never, Workflow> =>
	Workflow.use(({ rawRivetkitContext }) =>
		callRivetkit(() => rawRivetkitContext.rollbackCheckpoint(name)),
	);

/**
 * Waits for `duration`. The deadline is saved, so the wait continues after
 * the actor sleeps, and RivetKit wakes the actor when the deadline is reached.
 */
export const sleep = (
	name: string,
	duration: Duration.Input,
): Effect.Effect<void, never, Workflow> =>
	Workflow.use(({ rawRivetkitContext }) =>
		callRivetkit(() =>
			rawRivetkitContext.sleep(name, Duration.toMillis(duration)),
		),
	);

/** Waits until `time`. Durable like `sleep`. */
export const sleepUntil = (
	name: string,
	time: DateTime.DateTime | Date,
): Effect.Effect<void, never, Workflow> =>
	Workflow.use(({ rawRivetkitContext }) =>
		callRivetkit(() =>
			rawRivetkitContext.sleepUntil(
				name,
				DateTime.toEpochMillis(DateTime.makeUnsafe(time)),
			),
		),
	);

const LoopBreakTypeId = "~@rivetkit/effect/Workflow/LoopBreak";

export interface LoopBreak {
	readonly [LoopBreakTypeId]: typeof LoopBreakTypeId;
}

/** Succeed with this from the body of `loop` to end the loop. */
export const loopBreak: LoopBreak = { [LoopBreakTypeId]: LoopBreakTypeId };

/**
 * Makes what the `run` of a loop with state succeeds with:
 * `Loop.continue(state)` runs the next iteration with `state`, and
 * `Loop.break(value)` ends the loop with `value`.
 */
export { Loop };

/** What the `run` of a loop with state succeeds with. */
export type LoopResult<S, A> = WorkflowsPackage.LoopResult<S, A>;

/**
 * Runs a loop. RivetKit saves the iteration the loop has reached and removes
 * the history of old iterations, so a loop can run forever. Names of steps
 * and sleeps in an iteration must be unique only within that iteration.
 *
 * With a body, the loop runs `body` again and again until it succeeds with
 * `Workflow.loopBreak`.
 *
 * With `state` and `run`, each iteration runs `run` with the state of the
 * loop, which starts as `state`. `run` succeeds with `Loop.continue(next)`
 * to run again with `next`, or with `Loop.break(value)` to end the loop, and
 * the loop succeeds with `value`. RivetKit saves the state after each
 * iteration, so the loop continues with it after the actor sleeps. The state
 * and `value` must be JSON-compatible.
 *
 * @example
 * ```ts
 * yield* Workflow.loop(
 *   "poll",
 *   Effect.gen(function* () {
 *     const done = yield* Workflow.step("check", checkJob);
 *     if (done) return Workflow.loopBreak;
 *     yield* Workflow.sleep("wait", "1 minute");
 *   }),
 * );
 *
 * const total = yield* Workflow.loop("sum-pages", {
 *   state: { page: 0, total: 0 },
 *   run: ({ page, total }) =>
 *     Effect.gen(function* () {
 *       const count = yield* Workflow.step("count", countPage(page));
 *       if (count === 0) return Workflow.Loop.break(total);
 *       return Workflow.Loop.continue({ page: page + 1, total: total + count });
 *     }),
 * });
 * ```
 */
export const loop: {
	<R>(
		name: string,
		// biome-ignore lint/suspicious/noConfusingVoidType: a body that returns nothing continues the loop
		body: Effect.Effect<void | LoopBreak, never, R>,
	): Effect.Effect<void, never, R | Workflow>;
	<S, A, R>(
		name: string,
		options: {
			readonly state: S;
			readonly run: (
				state: S,
			) => Effect.Effect<LoopResult<NoInfer<S>, A>, never, R>;
		},
	): Effect.Effect<A, never, R | Workflow>;
} = (
	name: string,
	bodyOrOptions:
		| Effect.Effect<unknown, never, any>
		| {
				readonly state: unknown;
				readonly run: (
					state: unknown,
				) => Effect.Effect<LoopResult<unknown, unknown>, never, any>;
		  },
): Effect.Effect<any, never, any> =>
	Effect.gen(function* () {
		const { rawRivetkitContext } = yield* Workflow;
		const runPromise = runPromiseWith(yield* Effect.context<any>());

		if (Effect.isEffect(bodyOrOptions)) {
			const body = bodyOrOptions;
			return yield* callRivetkit(() =>
				rawRivetkitContext.loop(name, async (iterationContext) =>
					(await runPromise(
						provideRivetkitContext(body, iterationContext),
					)) === loopBreak
						? Loop.break(undefined)
						: Loop.continue(undefined),
				),
			);
		}

		const { state, run } = bodyOrOptions;
		return yield* callRivetkit(() =>
			rawRivetkitContext.loop({
				name,
				state,
				run: (iterationContext, iterationState) =>
					runPromise(
						provideRivetkitContext(
							run(iterationState),
							iterationContext,
						),
					),
			}),
		);
	});

type Branches = Readonly<Record<string, Effect.Effect<unknown, never, any>>>;

/**
 * Runs the branches at the same time and waits for all of them. Succeeds with
 * the result of each branch by name. If a branch fails, the others still run
 * to the end, and then the workflow fails.
 *
 * Each branch has its own history, so names must be unique only within one
 * branch. RivetKit saves the result of a finished branch, which must be
 * JSON-compatible, and does not run that branch again.
 *
 * @example
 * ```ts
 * const { stock, payment } = yield* Workflow.join("checks", {
 *   stock: Workflow.step("reserve-stock", reserveStock),
 *   payment: Workflow.step("authorize-payment", authorizePayment),
 * });
 * ```
 */
export const join: <const B extends Branches>(
	name: string,
	branches: B,
) => Effect.Effect<
	{ readonly [Name in keyof B]: Effect.Success<B[Name]> },
	never,
	Effect.Services<B[keyof B]> | Workflow
> = (name: string, branches: Branches): Effect.Effect<any, never, any> =>
	Effect.gen(function* () {
		const { rawRivetkitContext } = yield* Workflow;
		const runPromise = runPromiseWith(yield* Effect.context<any>());

		return yield* callRivetkit(() =>
			rawRivetkitContext.join(
				name,
				Object.fromEntries(
					Object.entries(branches).map(([branchName, body]) => [
						branchName,
						{
							run: (branchContext: RivetkitWorkflowContext) =>
								runPromise(
									provideRivetkitContext(body, branchContext),
								),
						},
					]),
				),
			),
		);
	});

/**
 * Runs a branch of `race` and interrupts it when `loserSignal` aborts, which
 * RivetKit does when another branch wins, or when the actor tears down.
 */
const runRaceBranch = <A, R>(
	context: Context.Context<R>,
	effect: Effect.Effect<A, never, R>,
	loserSignal: AbortSignal,
): Promise<A> =>
	new Promise((resolve, reject) => {
		const teardownSignal = Context.get(context, WorkflowTeardownSignal);
		Effect.runForkWith(context)(effect, {
			signal: AbortSignal.any([loserSignal, teardownSignal]),
		}).addObserver((exit) => {
			if (Exit.isSuccess(exit)) {
				resolve(exit.value);
			} else if (loserSignal.aborted) {
				reject(new CancelledError());
			} else if (
				teardownSignal.aborted &&
				Cause.hasInterruptsOnly(exit.cause)
			) {
				reject(teardownSignal.reason);
			} else {
				reject(Cause.squash(exit.cause));
			}
		});
	});

/**
 * Runs the branches at the same time. Succeeds with the name and the result
 * of the first branch that finishes. If all branches fail, the workflow
 * fails.
 *
 * The other branches are interrupted and RivetKit removes their history. A
 * step attempt that is in progress finishes first. The result of the winner
 * is saved and must be JSON-compatible.
 *
 * @example
 * ```ts
 * const result = yield* Workflow.race("approval", {
 *   approved: Workflow.step("wait-for-approval", waitForApproval),
 *   expired: Workflow.sleep("approval-deadline", "1 day"),
 * });
 * if (result.winner === "expired") return;
 * ```
 */
export const race: <const B extends Branches>(
	name: string,
	branches: B,
) => Effect.Effect<
	{
		readonly [Name in keyof B]: {
			readonly winner: Name;
			readonly value: Effect.Success<B[Name]>;
		};
	}[keyof B],
	never,
	Effect.Services<B[keyof B]> | Workflow
> = (name: string, branches: Branches): Effect.Effect<any, never, any> =>
	Effect.gen(function* () {
		const { rawRivetkitContext } = yield* Workflow;
		const context = yield* Effect.context<any>();

		return yield* callRivetkit(() =>
			rawRivetkitContext.race(
				name,
				Object.entries(branches).map(([branchName, body]) => ({
					name: branchName,
					run: (branchContext: RivetkitWorkflowContext) =>
						runRaceBranch(
							context,
							provideRivetkitContext(body, branchContext),
							branchContext.abortSignal,
						),
				})),
			),
		);
	});

/**
 * Gates a change to a workflow that has running instances. A workflow that
 * reaches this call for the first time gets `latest`. A workflow that ran
 * past this point before the call existed gets 1. The version is saved, so a
 * workflow always gets the same one.
 *
 * @example
 * ```ts
 * const version = yield* Workflow.getVersion("fraud-check", 2);
 * if (version >= 2) yield* Workflow.step("check-fraud", checkFraud);
 * ```
 */
export const getVersion = (
	name: string,
	latest: number,
): Effect.Effect<number, never, Workflow> =>
	Workflow.use(({ rawRivetkitContext }) =>
		callRivetkit(() => rawRivetkitContext.getVersion(name, latest)),
	);

/**
 * Stands in for a call that you deleted from the workflow, so the history of
 * the workflows that ran it still matches. `originalType` is the kind of the
 * deleted call, for example `"step"` or `"sleep"`.
 */
export const removed = (
	name: string,
	originalType: WorkflowsPackage.EntryKindType,
): Effect.Effect<void, never, Workflow> =>
	Workflow.use(({ rawRivetkitContext }) =>
		callRivetkit(() => rawRivetkitContext.removed(name, originalType)),
	);
