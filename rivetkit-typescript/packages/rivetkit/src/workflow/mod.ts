import { ACTOR_CONTEXT_INTERNAL_SYMBOL } from "@/actor/contexts/base/actor";
import type { RunContext } from "@/actor/contexts/run";
import type { AnyDatabaseProvider } from "@/actor/database";
import type { AnyActorInstance } from "@/actor/instance/mod";
import type { EventSchemaConfig, QueueSchemaConfig } from "@/actor/schema";
import { RUN_FUNCTION_CONFIG_SYMBOL } from "@/actor/config";
import { stringifyError } from "@/utils";
import {
	CriticalError,
	EntryInProgressError,
	HistoryDivergedError,
	JoinError,
	RaceError,
	rerunWorkflowFromStep,
	RollbackCheckpointError,
	RollbackError,
	runWorkflow,
	StepExhaustedError,
	type WorkflowErrorEvent,
} from "@rivetkit/workflow-engine";
import invariant from "invariant";
import { ActorWorkflowContext } from "./context";
import { ActorWorkflowControlDriver, ActorWorkflowDriver } from "./driver";
import { createWorkflowInspectorAdapter } from "./inspector";

export { Loop } from "@rivetkit/workflow-engine";
export type {
	WorkflowError,
	WorkflowErrorEvent,
} from "@rivetkit/workflow-engine";
export {
	ActorWorkflowContext,
	type WorkflowBranchContextOf,
	type WorkflowContextOf,
	type WorkflowLoopContextOf,
	type WorkflowStepContextOf,
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
		ctx: ActorWorkflowContext<
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

	const workflowInspectors = new WeakMap<
		AnyActorInstance,
		ReturnType<typeof createWorkflowInspectorAdapter>
	>();

	function getWorkflowInspector(actor: AnyActorInstance) {
		let workflowInspector = workflowInspectors.get(actor);
		if (!workflowInspector) {
			workflowInspector = createWorkflowInspectorAdapter();
			workflowInspectors.set(actor, workflowInspector);
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
				[ACTOR_CONTEXT_INTERNAL_SYMBOL]?: AnyActorInstance;
			}
		)[ACTOR_CONTEXT_INTERNAL_SYMBOL];
		invariant(actor, "workflow() requires an actor instance");
		const workflowInspector = getWorkflowInspector(actor);

		const driver = new ActorWorkflowDriver(actor, runCtx);
		workflowInspector.setRerunFromStep(async (entryId) => {
			const snapshot = await rerunWorkflowFromStep(
				actor.id,
				new ActorWorkflowControlDriver(actor),
				entryId,
				{ scheduleAlarm: false },
			);
			workflowInspector.update(snapshot);
			await actor.restartRunHandler();
			return workflowInspector.adapter.getHistory();
		});

		const handle = runWorkflow(
			actor.id,
			async (ctx) => await fn(new ActorWorkflowContext(ctx, runCtx)),
			undefined,
			driver,
			{
				mode: "live",
				logger: runCtx.log,
				onHistoryUpdated: workflowInspector.update,
				onError: onError
					? async (event) => await onError(runCtx, event)
					: undefined,
			},
		);

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
			if (runCtx.abortSignal.aborted) {
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

	const runWithConfig = run as typeof run & {
		[RUN_FUNCTION_CONFIG_SYMBOL]?: {
			icon?: string;
			inspector?: { workflow: typeof workflowInspector.adapter };
		};
	};
	runWithConfig[RUN_FUNCTION_CONFIG_SYMBOL] = {
		icon: "diagram-project",
		inspectorFactory: (actor) => {
			if (!actor) {
				return undefined;
			}
			return {
				workflow: getWorkflowInspector(actor as AnyActorInstance).adapter,
			};
		},
	};

	return runWithConfig;
}
