/**
 * The workflow actor runtime. One actor holds one workflow execution: its
 * payload, run result, activity results keyed `${name}/${attempt}`, durable
 * deferred exits, and the clock due table. A crash or sleep wipes memory, and
 * the next wake replays the execution from SQLite.
 *
 * Storage is asynchronous, so every operation that reads and then writes the
 * execution row, or reads and then changes the in-memory attempt, runs under
 * one entry permit. Work that waits on an attempt is always awaited after the
 * permit is released.
 *
 * Portions adapted from Effect (MIT License,
 * Copyright (c) 2023 Effectful Technologies Inc). See `LICENSE-effect`.
 *
 * @internal
 */
import * as Cause from "effect/Cause";
import * as Context from "effect/Context";
import * as Duration from "effect/Duration";
import * as Effect from "effect/Effect";
import * as Exit from "effect/Exit";
import * as Fiber from "effect/Fiber";
import * as Schedule from "effect/Schedule";
import * as Scope from "effect/Scope";
import * as Semaphore from "effect/Semaphore";
import * as DurableClock from "effect/workflow/DurableClock";
import * as Workflow from "effect/workflow/Workflow";
import * as WorkflowEngine from "effect/workflow/WorkflowEngine";
import { armTimer } from "./entityStorage.ts";
import type { ActorSql } from "./sql.ts";
import type { ActorTimer } from "./timer.ts";
import {
	CurrentExecutionHandle,
	type WorkflowExecutionHandle,
	type WorkflowRegistry,
	type WorkflowRunOptions,
	type WorkflowStub,
} from "./workflowRegistry.ts";
import * as WorkflowStorage from "./workflowStorage.ts";
import {
	decodePayload,
	encodeDefectResult,
	encodeExit,
	encodeResult,
} from "./workflowWire.ts";

/** @internal */
export const InterruptSignalName = "Workflow/InterruptSignal";

// While a resume is pending a guard wake stays armed this far ahead, so a
// replay lost with the process is retried instead of waiting for contact.
const resumeGuardMillis = 60_000;

/** The deferred state shared by one engine and the executions it hosts. */
export type DeferredState = ReturnType<typeof WorkflowEngine.makeDeferredState>;

let voidExitCache: Promise<string> | undefined;
const voidExitText = (): Promise<string> => {
	voidExitCache ??= Effect.runPromise(encodeExit(Exit.void, Context.empty()));
	return voidExitCache;
};

const mergeWake = (
	current: WorkflowStorage.WakeMode | undefined,
	mode: WorkflowStorage.WakeMode,
): WorkflowStorage.WakeMode => (current === "explicit" ? current : mode);

/** @internal */
export interface WorkflowRuntimeOptions {
	readonly workflowName: string;
	readonly executionId: string;
	readonly registry: WorkflowRegistry;
	readonly deferredState: DeferredState;
	readonly sql: ActorSql;
	readonly timer: ActorTimer;
	/** Keeps the actor awake until the promise settles. */
	readonly keepAwake: (promise: Promise<unknown>) => void;
	readonly getStub: (
		workflowName: string,
		executionId: string,
	) => WorkflowStub;
}

/** @internal */
export interface WorkflowRuntime extends WorkflowExecutionHandle {
	readonly runTimer: () => Promise<void>;
	/** Replays an execution whose pending resume was lost with the last process. */
	readonly wake: () => Promise<void>;
	/** Interrupts the running attempt without recording a result, for a stopping actor. */
	readonly shutdown: () => Promise<void>;
	readonly isStopping: () => boolean;
}

interface Inflight {
	readonly instance: WorkflowEngine.WorkflowInstance["Service"];
	readonly fiber: Fiber.Fiber<
		{
			readonly result: Workflow.Result<unknown, unknown>;
			readonly text: string;
		},
		unknown
	>;
	readonly promise: Promise<string>;
}

interface Attempt {
	readonly result: Workflow.Result<unknown, unknown>;
	readonly text: string;
}

class WorkflowStoppingError extends Error {
	constructor() {
		super("Workflow actor is stopping");
		this.name = "WorkflowStoppingError";
	}
}

/** @internal */
export const makeWorkflowRuntime = (
	options: WorkflowRuntimeOptions,
): WorkflowRuntime => {
	const { workflowName, executionId, sql, deferredState } = options;
	const entry = Semaphore.makeUnsafe(1);
	const withEntry = <A, E>(effect: Effect.Effect<A, E>) =>
		Semaphore.withPermit(entry, effect);

	let inflight: Inflight | undefined;
	let resumeRequested: WorkflowStorage.WakeMode | undefined;
	let stopping = false;
	// Parent of every attempt's scope. Scopes of attempts that suspended stay
	// open under it, so their finalizers and resources last until the
	// execution completes or an attempt defects.
	let executionScope = Scope.makeUnsafe();

	// The final attempt's scope has already closed with the real exit. Closing
	// the parent closes earlier attempts' scopes in reverse order with
	// `Exit.void`, so their superseded compensations are skipped. A fresh
	// parent is swapped in first: a later replay must not fork from a closed
	// scope, which would run its finalizers as soon as they are registered.
	const closeExecutionScope = Effect.suspend(() => {
		const closing = executionScope;
		executionScope = Scope.makeUnsafe();
		return Scope.close(closing, Exit.void);
	});

	const detach = (promise: Promise<unknown>): void => {
		options.keepAwake(
			promise.then(
				() => undefined,
				() => undefined,
			),
		);
	};

	const parseResult = (result: string) =>
		JSON.parse(result) as {
			readonly _tag?: unknown;
			readonly failed?: unknown;
		};

	const isComplete = (result: string | undefined): boolean =>
		result !== undefined && parseResult(result)._tag === "Complete";

	// Only an explicit wake replays a failure suspension.
	const isUnwakeable = (
		result: string | undefined,
		mode: WorkflowStorage.WakeMode,
	): boolean => {
		if (result === undefined) return false;
		const parsed = parseResult(result);
		return (
			parsed._tag === "Complete" ||
			(mode === "automatic" && parsed.failed === true)
		);
	};

	/**
	 * Records a wake durably. A running attempt queues it and decides when it
	 * settles, since its result may still become a failure suspension. Returns
	 * whether a replay is now pending. Runs under the entry permit.
	 */
	const requestWake = (mode: WorkflowStorage.WakeMode) =>
		Effect.gen(function* () {
			const row = yield* WorkflowStorage.loadExecution(sql);
			if (row === undefined || isComplete(row.result)) return false;
			if (inflight === undefined && isUnwakeable(row.result, mode)) {
				return false;
			}
			const pending = mergeWake(row.resumePending, mode);
			if (pending !== row.resumePending) {
				yield* WorkflowStorage.setResumePending(sql, pending);
			}
			if (inflight !== undefined) {
				resumeRequested = mergeWake(resumeRequested, mode);
			}
			return true;
		});

	const armResumeGuard = Effect.suspend(() =>
		armTimer(options.timer, Date.now() + resumeGuardMillis),
	);

	const resumeParent = (parent: WorkflowStorage.ExecutionParent) =>
		Effect.promise(() =>
			options
				.getStub(parent.workflowName, parent.executionId)
				.resume("automatic"),
		).pipe(
			Effect.sandbox,
			Effect.retry({ times: 5, schedule: Schedule.exponential(200) }),
		);

	const deliverParentWake: Effect.Effect<void> = Effect.gen(function* () {
		const row = yield* withEntry(WorkflowStorage.loadExecution(sql));
		if (
			row === undefined ||
			row.resumePending === undefined ||
			!isComplete(row.result) ||
			row.parent === undefined
		) {
			return;
		}
		// Re-arm before sending so a crash or exhausted retries leave another attempt.
		yield* armResumeGuard;
		yield* resumeParent(row.parent).pipe(
			Effect.andThen(
				withEntry(WorkflowStorage.setResumePending(sql, undefined)),
			),
			Effect.catchCause((cause) =>
				Effect.logError("Workflow parent resume failed", cause),
			),
		);
	});

	// An attempt the machinery cannot finish (a defect with `CaptureDefects`
	// off, a payload that no longer decodes, a result that fails to encode, a
	// dying finalizer, a missing registration) completes the execution with the
	// defect. The actor never rejects `run` for such a reason, so a caller can
	// tell the child's own defect from a transport failure.
	const failAttempt = (cause: Cause.Cause<unknown>): Promise<string> =>
		Effect.runPromise(
			withEntry(
				Effect.gen(function* () {
					yield* Effect.logError("Workflow execution failed", cause);
					const text = yield* encodeDefectResult(Cause.squash(cause));
					const parent = (yield* WorkflowStorage.loadExecution(sql))
						?.parent;
					if (parent !== undefined) yield* armResumeGuard;
					yield* WorkflowStorage.saveCompletion(
						sql,
						text,
						parent !== undefined,
					);
					inflight = undefined;
					resumeRequested = undefined;
					if (parent !== undefined)
						detach(Effect.runPromise(deliverParentWake));
					return text;
				}),
			),
		);

	/** Starts an attempt. Called under the entry permit; never awaited under it. */
	const startAttempt = (
		row: WorkflowStorage.ExecutionRow,
	): Promise<string> => {
		const registration = options.registry.get(workflowName);
		if (registration === undefined) {
			return failAttempt(
				Cause.die(
					new Error(
						`No workflow registered for name: ${workflowName}`,
					),
				),
			);
		}
		const workflow = registration.workflow;
		const instance = WorkflowEngine.WorkflowInstance.initial(
			workflow,
			executionId,
			{ scope: Scope.forkUnsafe(executionScope) },
		);
		const execute = decodePayload(
			workflow,
			row.payload,
			registration.context,
		).pipe(
			Effect.flatMap(
				(payload) =>
					registration.execute(payload, executionId) as Effect.Effect<
						unknown,
						unknown
					>,
			),
			Effect.onExit((exit) => {
				const suspendOnFailure = Context.get(
					workflow.annotations,
					Workflow.SuspendOnFailure,
				);
				if (
					!instance.suspended &&
					!(suspendOnFailure && exit._tag === "Failure")
				) {
					return Effect.void;
				}
				return Effect.flatMap(
					WorkflowStorage.loadDeferred(sql, InterruptSignalName),
					(signal) => {
						if (signal === undefined) return Effect.void;
						instance.suspended = false;
						instance.interrupted = true;
						return Effect.withFiber<void>((fiber) =>
							Effect.interruptible(Fiber.interrupt(fiber)),
						);
					},
				);
			}),
			Workflow.intoResult,
			(effect) => deferredState.trackRun(instance, effect),
			Effect.flatMap((result) =>
				Effect.map(
					encodeResult(workflow, result, registration.context),
					(text) => ({ result, text }),
				),
			),
			Effect.provideService(
				DurableClock.InMemoryThreshold,
				Duration.zero,
			),
			Effect.provideService(CurrentExecutionHandle, runtime),
		);

		const persist = ({ result, text }: Attempt): Promise<string> =>
			Effect.runPromise(
				withEntry(
					Effect.gen(function* () {
						const parent = (yield* WorkflowStorage.loadExecution(
							sql,
						))?.parent;
						if (result._tag === "Complete") {
							// Arm before the result and pending wake are saved, so a
							// persisted completion always has a delivery wake.
							if (parent !== undefined) yield* armResumeGuard;
							yield* WorkflowStorage.saveCompletion(
								sql,
								text,
								parent !== undefined,
							);
							inflight = undefined;
							resumeRequested = undefined;
							if (parent !== undefined) {
								detach(Effect.runPromise(deliverParentWake));
							}
							return text;
						}
						const replay =
							resumeRequested !== undefined &&
							!isUnwakeable(text, resumeRequested)
								? resumeRequested
								: undefined;
						yield* WorkflowStorage.saveSuspension(
							sql,
							text,
							replay,
						);
						inflight = undefined;
						resumeRequested = undefined;
						if (replay !== undefined) detach(startAttempt(row));
						return text;
					}).pipe(
						Effect.tapCause(() =>
							Effect.sync(() => {
								inflight = undefined;
							}),
						),
					),
				),
			);

		const finish = (attempt: Attempt): Promise<string> => {
			if (attempt.result._tag !== "Complete") return persist(attempt);
			// Close retained scopes before saving, so a durable completion never
			// has unrun finalizers. A dying finalizer fails the attempt instead.
			return Effect.runPromise(Effect.exit(closeExecutionScope)).then(
				(closed) =>
					Exit.isSuccess(closed)
						? persist(attempt)
						: failAttempt(closed.cause),
			);
		};

		const fiber = Effect.runFork(execute);
		const promise = Effect.runPromise(Fiber.await(fiber)).then((exit) => {
			if (stopping) {
				// The actor is stopping. The pending resume stays recorded, so the
				// execution replays on the next start instead of failing here.
				inflight = undefined;
				throw new WorkflowStoppingError();
			}
			if (Exit.isSuccess(exit)) return finish(exit.value);
			if (
				Cause.hasInterruptsOnly(exit.cause) &&
				(instance.interrupted || instance.suspended)
			) {
				// A hard interrupt, or a completion preempting the fiber after it
				// produced its result, kills the fiber before it can encode, so the
				// result is synthesized from the instance state.
				const result: Workflow.Result<unknown, unknown> =
					instance.interrupted
						? new Workflow.Complete({
								exit: exit as Exit.Exit<unknown, unknown>,
							})
						: new Workflow.Suspended({});
				return Effect.runPromise(
					encodeResult(workflow, result, registration.context),
				).then((text) => finish({ result, text }));
			}
			// A defect ends every retained attempt; a replay starts in fresh scopes.
			return Effect.runPromise(Effect.exit(closeExecutionScope)).then(
				(closed) =>
					failAttempt(
						Exit.isSuccess(closed)
							? exit.cause
							: Cause.combine(exit.cause, closed.cause),
					),
			);
		});
		inflight = { instance, fiber, promise };
		return promise;
	};

	const run = (payload: string, opts: WorkflowRunOptions): Promise<string> =>
		Effect.runPromise(
			withEntry(
				Effect.gen(function* () {
					if (stopping)
						return Promise.reject(new WorkflowStoppingError());
					let row = yield* WorkflowStorage.loadExecution(sql);
					if (row === undefined) {
						yield* WorkflowStorage.createExecution(
							sql,
							workflowName,
							executionId,
							payload,
							opts.parent,
						);
						row = {
							workflowName,
							executionId,
							payload,
							parent: opts.parent,
							result: undefined,
							resumePending: undefined,
						};
					} else if (
						opts.parent !== undefined &&
						row.parent === undefined
					) {
						// An execution started standalone can gain a parent later; keep
						// the first parent so its completion still wakes that parent.
						yield* WorkflowStorage.setParent(sql, opts.parent);
						row = { ...row, parent: opts.parent };
					}
					if (inflight !== undefined) {
						return opts.discard
							? Promise.resolve("")
							: inflight.promise;
					}
					if (row.result !== undefined) {
						return Promise.resolve(opts.discard ? "" : row.result);
					}
					// The attempt stays pending under the guard wake until it persists
					// a result, so an attempt lost with the process is replayed even
					// when no caller awaits it.
					yield* WorkflowStorage.setResumePending(
						sql,
						mergeWake(row.resumePending, "automatic"),
					);
					yield* armResumeGuard;
					const attempt = startAttempt(row);
					if (opts.discard) {
						detach(attempt);
						return Promise.resolve("");
					}
					return attempt;
				}),
			),
		).then((attempt) => attempt);

	/** Starts the pending replay, if any. Runs under the entry permit. */
	const startPending = Effect.gen(function* () {
		const row = yield* WorkflowStorage.loadExecution(sql);
		if (
			row === undefined ||
			row.resumePending === undefined ||
			inflight !== undefined
		) {
			return;
		}
		if (isUnwakeable(row.result, row.resumePending)) return;
		detach(startAttempt(row));
	});

	const resume = (
		mode: WorkflowStorage.WakeMode = "explicit",
	): Promise<void> =>
		Effect.runPromise(
			withEntry(
				Effect.gen(function* () {
					if (!(yield* requestWake(mode))) return;
					// Persist and guard the resume before acknowledging it, so an abort
					// after the action returns is still recovered.
					yield* armResumeGuard;
					yield* startPending;
				}),
			),
		);

	const completeDeferred = (
		deferredName: string,
		exitText: string,
		mode: WorkflowStorage.WakeMode,
	): Promise<void> =>
		Effect.runPromise(
			withEntry(
				Effect.gen(function* () {
					const saved = yield* WorkflowStorage.saveDeferred(
						sql,
						deferredName,
						exitText,
					);
					if (!saved) return false;
					// Persisted with the exit: if this wake is lost before the replay
					// finishes, the next wake or the guard replays the execution.
					if (!(yield* requestWake(mode))) return false;
					yield* armResumeGuard;
					return true;
				}),
			),
		).then((woken) => {
			if (!woken) return undefined;
			return Effect.runPromise(
				deferredState.deferredDone(executionId, deferredName),
			).then(() => resume(mode));
		});

	const deferredDone = (
		deferredName: string,
		exitText: string,
	): Promise<void> => completeDeferred(deferredName, exitText, "automatic");

	// The caller belongs to the run this wake interrupts, so awaiting that run
	// would wait on the caller's own cleanup. The wake starts on a separate
	// fiber after the caller continues.
	const completeOwnDeferred = (
		deferredName: string,
		exitText: string,
	): Effect.Effect<void> =>
		Effect.gen(function* () {
			const woken = yield* withEntry(
				Effect.gen(function* () {
					const saved = yield* WorkflowStorage.saveDeferred(
						sql,
						deferredName,
						exitText,
					);
					if (!saved) return false;
					if (!(yield* requestWake("automatic"))) return false;
					yield* armResumeGuard;
					return true;
				}),
			);
			if (!woken) return;
			const wake = deferredState
				.deferredDone(executionId, deferredName)
				.pipe(
					Effect.andThen(
						Effect.suspend(() =>
							inflight === undefined
								? Effect.promise(() => resume("automatic"))
								: Effect.void,
						),
					),
				);
			const fiber = yield* Effect.forkDetach(wake);
			detach(Effect.runPromise(Fiber.await(fiber)));
		});

	const interrupt = (): Promise<void> =>
		Effect.runPromise(withEntry(WorkflowStorage.loadExecution(sql))).then(
			(row) => {
				if (row === undefined || isComplete(row.result))
					return undefined;
				return voidExitText().then((exitText) =>
					completeDeferred(InterruptSignalName, exitText, "explicit"),
				);
			},
		);

	const interruptUnsafe = (): Promise<void> => {
		const current = inflight;
		const signalled = interrupt();
		if (current === undefined) return signalled;
		current.instance.interrupted = true;
		return signalled
			.then(() => Effect.runPromise(Fiber.interrupt(current.fiber)))
			.then(() => undefined);
	};

	const scheduleClock = (
		clockName: string,
		deferredName: string,
		wakeUp: number,
	): Promise<void> =>
		Effect.runPromise(
			withEntry(
				Effect.gen(function* () {
					yield* WorkflowStorage.saveClock(
						sql,
						clockName,
						deferredName,
						wakeUp,
					);
					const earliest =
						yield* WorkflowStorage.earliestClockWakeUp(sql);
					if (earliest !== undefined)
						yield* armTimer(options.timer, earliest);
				}),
			),
		);

	const runTimer = async (): Promise<void> => {
		const voidExit = await voidExitText();
		const completed = await Effect.runPromise(
			withEntry(
				Effect.gen(function* () {
					yield* options.timer.fired;
					const due = yield* WorkflowStorage.dueClocks(
						sql,
						Date.now(),
					);
					const fired: Array<{ readonly deferredName: string }> = [];
					for (const clock of due) {
						yield* WorkflowStorage.markClockFired(sql, clock.name);
						const saved = yield* WorkflowStorage.saveDeferred(
							sql,
							clock.deferredName,
							voidExit,
						);
						if (saved) fired.push(clock);
					}
					if (fired.length > 0) yield* requestWake("automatic");
					return fired;
				}),
			),
		);
		await Effect.runPromise(
			Effect.forEach(
				completed,
				(clock) =>
					deferredState.deferredDone(executionId, clock.deferredName),
				{ discard: true },
			),
		);
		const row = await Effect.runPromise(
			withEntry(WorkflowStorage.loadExecution(sql)),
		);
		const mode = row?.resumePending;
		const pending = mode !== undefined && !isUnwakeable(row?.result, mode);
		// A live attempt is not lost: only re-arm the guard. Requesting a wake
		// would replay the attempt once it suspends, for no reason.
		if (pending && inflight === undefined) await resume(mode);
		await Effect.runPromise(
			withEntry(
				Effect.gen(function* () {
					// While a resume is pending a guard wake stays armed, so a replay
					// lost with this process is retried instead of sleeping forever.
					const earliest =
						yield* WorkflowStorage.earliestClockWakeUp(sql);
					const targets = [
						earliest,
						pending ? Date.now() + resumeGuardMillis : undefined,
					].filter((target) => target !== undefined);
					if (targets.length === 0)
						return yield* options.timer.delete;
					// The wake that fired is spent, so set the next one outright.
					yield* options.timer.set(Math.min(...targets));
				}),
			),
		);
		await Effect.runPromise(deliverParentWake);
	};

	const wake = async (): Promise<void> => {
		const earliest = await Effect.runPromise(
			withEntry(WorkflowStorage.earliestClockWakeUp(sql)),
		);
		if (earliest !== undefined) {
			await Effect.runPromise(
				withEntry(armTimer(options.timer, earliest)),
			);
		}
		// A resume recorded before the last process stopped replays now.
		const row = await Effect.runPromise(
			withEntry(WorkflowStorage.loadExecution(sql)),
		);
		if (row?.resumePending !== undefined) await resume(row.resumePending);
	};

	const shutdown = async (): Promise<void> => {
		stopping = true;
		const current = inflight;
		if (current === undefined) return;
		await Effect.runPromise(Fiber.interrupt(current.fiber));
	};

	const runtime: WorkflowRuntime = {
		executionId,
		run,
		poll: () =>
			Effect.runPromise(
				withEntry(WorkflowStorage.loadExecution(sql)),
			).then((row) => row?.result),
		resume,
		interrupt,
		interruptUnsafe,
		deferredDone,
		scheduleClock,
		runTimer,
		wake,
		shutdown,
		isStopping: () => stopping,
		loadActivity: (key) => WorkflowStorage.loadActivity(sql, key),
		saveActivity: (key, exit) =>
			WorkflowStorage.saveActivity(sql, key, exit),
		loadDeferred: (deferredName) =>
			WorkflowStorage.loadDeferred(sql, deferredName),
		completeOwnDeferred,
	};

	return runtime;
};
