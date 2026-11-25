import { EvictedError } from "@rivet-dev/workflows";
import { Cause, Context, Effect, Exit } from "effect";
import { type RivetkitWorkflowContext, Workflow } from "../Workflow.ts";
import { makeActorLogAnnotations } from "./logging.ts";
import { WorkflowTeardownSignal } from "./WorkflowTeardownSignal.ts";

type Instance = {
	readonly context: Context.Context<any>;
	readonly workflowBody: Effect.Effect<void, never, any> | undefined;
};

/**
 * The workflow engine evicts a workflow by aborting this signal with its own
 * eviction error, the only error that stops a run without failing the
 * workflow.
 */
const rejectWhenEvicted = (abortSignal: AbortSignal) =>
	new Promise<never>((_, reject) => {
		if (abortSignal.aborted) return reject(abortSignal.reason);
		abortSignal.addEventListener(
			"abort",
			() => reject(abortSignal.reason),
			{ once: true },
		);
	});

/**
 * Builds the `run` of a `@rivet-dev/workflows` workflow actor. Each call runs
 * the workflow body of the wake's instance, which that wake built. RivetKit
 * starts it after `onWake`, so the instance exists unless the wake did not
 * complete or the actor is already shutting down. Without an
 * instance the call only waits for the eviction, which saves nothing.
 *
 * The workflow engine controls a run with errors that it throws from its own
 * calls, for example to wait for a durable sleep. They cross the effect as
 * defects and are thrown again here unchanged.
 *
 * A sleep evicts the workflow but does not interrupt it: RivetKit waits for
 * the run during the sleep grace period, and the engine stops the workflow at
 * its next call. RivetKit does not report the end of the grace period, so a
 * timer of `getSleepGracePeriodMillis` interrupts a run that is still going
 * then. `interruptOnDestroy` does it at once, because a destroy does not wait.
 */
export const makeWorkflowRun = ({
	getInstance,
	getSleepGracePeriodMillis,
}: {
	readonly getInstance: (actorId: string) => Instance | undefined;
	readonly getSleepGracePeriodMillis: () => number;
}) => {
	const teardownByActorId = new Map<string, AbortController>();

	const run = async (
		rawRivetkitContext: RivetkitWorkflowContext,
	): Promise<void> => {
		const { abortSignal, actorId } = rawRivetkitContext;
		const instance = getInstance(actorId);
		if (instance?.workflowBody === undefined) {
			return rejectWhenEvicted(abortSignal);
		}

		const teardown = new AbortController();
		teardownByActorId.set(actorId, teardown);
		let gracePeriodTimer: ReturnType<typeof setTimeout> | undefined;
		const onEvicted = () => {
			gracePeriodTimer = setTimeout(
				() => teardown.abort(abortSignal.reason),
				getSleepGracePeriodMillis(),
			);
		};
		if (abortSignal.aborted) onEvicted();
		else abortSignal.addEventListener("abort", onEvicted, { once: true });
		const stopWatchingTeardown = () => {
			clearTimeout(gracePeriodTimer);
			abortSignal.removeEventListener("abort", onEvicted);
			if (teardownByActorId.get(actorId) === teardown) {
				teardownByActorId.delete(actorId);
			}
		};
		teardown.signal.addEventListener("abort", stopWatchingTeardown, {
			once: true,
		});

		const exit = await Effect.runPromiseExitWith(instance.context)(
			instance.workflowBody.pipe(
				Effect.provide(
					Context.make(Workflow, { rawRivetkitContext }).pipe(
						Context.add(WorkflowTeardownSignal, teardown.signal),
					),
				),
				Effect.annotateLogs(
					makeActorLogAnnotations(rawRivetkitContext),
				),
			),
			{ signal: teardown.signal },
		);
		stopWatchingTeardown();
		if (Exit.isSuccess(exit)) return;
		if (Cause.hasInterruptsOnly(exit.cause)) {
			return rejectWhenEvicted(abortSignal);
		}
		const error = Cause.squash(exit.cause);
		/**
		 * An interrupted step attempt stays pending, so the engine can still
		 * time it out after the teardown. Settling then would save that
		 * failure over the history of the next wake.
		 */
		if (teardown.signal.aborted && !(error instanceof EvictedError)) {
			return new Promise<never>(() => {});
		}
		throw error;
	};

	return {
		run,
		interruptOnDestroy: (actorId: string) => {
			teardownByActorId.get(actorId)?.abort(new EvictedError());
		},
	};
};
