import {
	CriticalError,
	EntryInProgressError,
	HistoryDivergedError,
	JoinError,
	RaceError,
	RollbackCheckpointError,
	RollbackError,
	type RunWorkflowOptions,
	replayWorkflowFromStep,
	runWorkflow,
	StepExhaustedError,
	type WorkflowErrorEvent,
} from "@rivetkit/workflow-engine";
import invariant from "invariant";
import {
	ACTOR_CONTEXT_INTERNAL_SYMBOL,
	defineRunHandler,
	type RunContext,
	type RunControl,
} from "@/actor/config";
import type { AnyStaticActorInstance } from "@/actor/definition";
import { isActorAbortedError, RivetError } from "@/actor/errors";
import type { EventSchemaConfig, QueueSchemaConfig } from "@/actor/schema";
import type { AnyDatabaseProvider } from "@/common/database/config";
import { stringifyError } from "@/utils";
import { throwIfGenerationLost, WorkflowContext } from "./context";
import { ActorWorkflowControlDriver, ActorWorkflowDriver } from "./driver";
import { createWorkflowInspectorAdapter } from "./inspector";

export type {
	TryBlockCatchKind,
	TryBlockConfig,
	TryBlockFailure,
	TryBlockResult,
	TryStepCatchKind,
	TryStepConfig,
	TryStepFailure,
	TryStepResult,
	WorkflowError,
	WorkflowErrorEvent,
} from "@rivetkit/workflow-engine";
export { Loop } from "@rivetkit/workflow-engine";
export {
	type WorkflowBranchConfig,
	type WorkflowBranchContextOf,
	WorkflowContext,
	type WorkflowContextOf,
	type WorkflowLoopConfig,
	type WorkflowLoopContextOf,
	type WorkflowStepConfig,
	WorkflowStepContext,
	type WorkflowStepContextOf,
	type WorkflowTryConfig,
	type WorkflowTryStepConfig,
} from "./context";

function shouldRethrowWorkflowError(error: unknown): boolean {
	if (
		error instanceof CriticalError ||
		error instanceof JoinError ||
		error instanceof RaceError ||
		error instanceof RollbackError ||
		error instanceof StepExhaustedError
	) {
		return false;
	}

	if (
		error instanceof EntryInProgressError ||
		error instanceof HistoryDivergedError ||
		error instanceof RollbackCheckpointError
	) {
		return true;
	}

	return true;
}

function workflowReplayInFlightError(): RivetError {
	return new RivetError(
		"actor",
		"workflow_in_flight",
		"Workflow replay is unavailable while the workflow is currently in flight.",
		{
			public: true,
			statusCode: 409,
		},
	);
}

function isWorkflowReplayBlockedByRunningEntry(error: unknown): boolean {
	return (
		error instanceof Error &&
		error.message ===
			"Cannot replay a workflow while a step is currently running"
	);
}

function isRunHandlerUnavailable(error: unknown): boolean {
	return (
		error instanceof RivetError &&
		error.group === "actor" &&
		error.code === "run_handler_unavailable"
	);
}

export interface WorkflowOptions<
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider,
	TEvents extends EventSchemaConfig = Record<never, never>,
	TQueues extends QueueSchemaConfig = Record<never, never>,
> {
	onError?: (
		ctx: RunContext<
			TState,
			TConnParams,
			TConnState,
			TVars,
			TInput,
			TDatabase,
			TEvents,
			TQueues
		>,
		event: WorkflowErrorEvent,
	) => void | Promise<void>;
}

export function workflow<
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider,
	TEvents extends EventSchemaConfig = Record<never, never>,
	TQueues extends QueueSchemaConfig = Record<never, never>,
>(
	fn: (
		ctx: WorkflowContext<
			TState,
			TConnParams,
			TConnState,
			TVars,
			TInput,
			TDatabase,
			TEvents,
			TQueues
		>,
	) => Promise<unknown>,
	options: WorkflowOptions<
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		TDatabase,
		TEvents,
		TQueues
	> = {},
): (
	c: RunContext<
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		TDatabase,
		TEvents,
		TQueues
	>,
) => Promise<void> {
	const onError = options.onError;
	type WorkflowInspectorRegistration = ReturnType<
		typeof createWorkflowInspectorAdapter
	> & { control?: RunControl };
	// Keyed by actor id and generation so a lost generation that is still running cannot rebind
	// the control or state of the generation that replaced it.
	const workflowInspectors = new Map<string, WorkflowInspectorRegistration>();

	function workflowInspectorKey(
		actorId: string,
		actorGeneration: number | undefined,
	): string {
		return actorGeneration === undefined
			? actorId
			: `${actorId}#${actorGeneration}`;
	}

	function getWorkflowInspector(
		actorId: string,
		actorGeneration: number | undefined,
	) {
		const key = workflowInspectorKey(actorId, actorGeneration);
		let workflowInspector = workflowInspectors.get(key);
		if (!workflowInspector) {
			workflowInspector = createWorkflowInspectorAdapter();
			workflowInspectors.set(key, workflowInspector);
		}
		return workflowInspector;
	}

	async function run(
		runCtx: RunContext<
			TState,
			TConnParams,
			TConnState,
			TVars,
			TInput,
			TDatabase,
			TEvents,
			TQueues
		>,
	): Promise<void> {
		const actor = (
			runCtx as unknown as {
				[ACTOR_CONTEXT_INTERNAL_SYMBOL]?: AnyStaticActorInstance;
			}
		)[ACTOR_CONTEXT_INTERNAL_SYMBOL];
		invariant(actor, "workflow() requires an actor instance");
		const actorGeneration = (actor as { generation?: number }).generation;
		const workflowInspector = getWorkflowInspector(
			actor.id,
			actorGeneration,
		);

		const driver = new ActorWorkflowDriver(actor, runCtx);
		const controlDriver = new ActorWorkflowControlDriver(actor, runCtx);
		workflowInspector.setReplayFromStep(async (entryId) => {
			const workflowState = await workflowInspector.adapter.getState();
			if (workflowState === "pending" || workflowState === "running") {
				throw workflowReplayInFlightError();
			}

			const control = workflowInspector.control;
			invariant(control, "workflow Inspector control is not initialized");
			try {
				return await control.run.withInactive(
					{ restartOnSuccess: true },
					async () => {
						const snapshot = await replayWorkflowFromStep(
							actor.id,
							controlDriver,
							entryId,
							{ scheduleAlarm: false },
						);
						workflowInspector.update(snapshot);
						return workflowInspector.adapter.getHistory();
					},
				);
			} catch (error) {
				if (
					isWorkflowReplayBlockedByRunningEntry(error) ||
					isRunHandlerUnavailable(error)
				) {
					throw workflowReplayInFlightError();
				}
				throw error;
			}
		});

		const handle = runWorkflow(
			actor.id,
			async (ctx) => {
				throwIfGenerationLost(runCtx);
				return await fn(new WorkflowContext(ctx, runCtx));
			},
			undefined,
			driver,
			{
				mode: "live",
				// The actor logger and the engine's pino logger are runtime
				// compatible but not structurally assignable.
				logger: runCtx.log as RunWorkflowOptions["logger"],
				onHistoryUpdated: workflowInspector.update,
				onError: onError
					? async (event) => {
							// A lost generation starts no new user hook.
							if (
								(actor as { isLost?: boolean }).isLost === true
							) {
								return;
							}
							await onError(runCtx, event);
						}
					: undefined,
			},
		);
		workflowInspector.setGetState(async () => await handle.getState());

		const onAbort = () => {
			handle.evict();
		};
		if (runCtx.abortSignal.aborted) {
			onAbort();
		} else {
			runCtx.abortSignal.addEventListener("abort", onAbort, {
				once: true,
			});
		}

		try {
			await handle.result;
		} catch (error) {
			// `abortSignal.aborted` is delivered on a separate async hop and
			// races the rejection, so detect the sleep abort structurally too.
			if (runCtx.abortSignal.aborted || isActorAbortedError(error)) {
				return;
			}

			if (shouldRethrowWorkflowError(error)) {
				runCtx.log.error({
					msg: "workflow run failed",
					error: stringifyError(error),
				});
				throw error;
			}

			runCtx.log.warn({
				msg: "workflow failed and will sleep until woken",
				error: stringifyError(error),
			});
		} finally {
			runCtx.abortSignal.removeEventListener("abort", onAbort);
		}
	}

	return defineRunHandler(run, {
		icon: "diagram-project",
		inspectorKind: "workflow",
		createInspector: ({ actorId, actorGeneration, control }) => {
			const key = workflowInspectorKey(actorId, actorGeneration);
			const workflowInspector = getWorkflowInspector(
				actorId,
				actorGeneration,
			);
			workflowInspector.control = control;
			return {
				inspector: { workflow: workflowInspector.adapter },
				dispose: () => {
					// Do not let a stale disposer remove a newly-created adapter for
					// the same actor generation.
					if (workflowInspectors.get(key) === workflowInspector) {
						workflowInspectors.delete(key);
					}
				},
			};
		},
	});
}
