// @ts-nocheck
import { ACTOR_CONTEXT_INTERNAL_SYMBOL } from "@/actor/config";
import type { RunContext } from "@/actor/config";
import type { AnyDatabaseProvider } from "@/common/database/config";
import type {
	AnyStaticActorInstance,
} from "@/actor/definition";
import type { EventSchemaConfig, QueueSchemaConfig } from "@/actor/schema";
import { RUN_FUNCTION_CONFIG_SYMBOL } from "@/actor/config";
import { stringifyError } from "@/utils";
import {
	CriticalError,
	EntryInProgressError,
	HistoryDivergedError,
	JoinError,
	RaceError,
	RollbackCheckpointError,
	RollbackError,
	runWorkflow,
	StepExhaustedError,
	type WorkflowErrorEvent,
} from "@rivetkit/workflow-engine";
import invariant from "invariant";
import { ActorWorkflowDriver } from "./driver";
import { ActorWorkflowContext } from "./context";

export { Loop } from "@rivetkit/workflow-engine";
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

		const driver = new ActorWorkflowDriver(actor, runCtx);

		const handle = runWorkflow(
			actor.id,
			async (ctx) => await fn(new ActorWorkflowContext(ctx, runCtx)),
			undefined,
			driver,
			{
				mode: "live",
				logger: runCtx.log,
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
		};
	};
	runWithConfig[RUN_FUNCTION_CONFIG_SYMBOL] = {
		icon: "diagram-project",
	};

	return runWithConfig;
}
