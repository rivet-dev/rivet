import { ACTOR_CONTEXT_INTERNAL_SYMBOL } from "@/actor/contexts/base/actor";
import type { RunContext } from "@/actor/contexts/run";
import type { AnyDatabaseProvider } from "@/actor/database";
import type { AnyActorInstance } from "@/actor/instance/mod";
import type { EventSchemaConfig, QueueSchemaConfig } from "@/actor/schema";
import { RUN_FUNCTION_CONFIG_SYMBOL } from "@/actor/config";
import { stringifyError } from "@/utils";
import { runWorkflow } from "@rivetkit/workflow-engine";
import invariant from "invariant";
import { ActorWorkflowContext } from "./context";
import { ActorWorkflowDriver } from "./driver";
import { createWorkflowInspectorAdapter } from "./inspector";

export { Loop } from "@rivetkit/workflow-engine";
export {
	ActorWorkflowContext,
	type WorkflowBranchContextOf,
	type WorkflowContextOf,
	type WorkflowLoopContextOf,
	type WorkflowStepContextOf,
} from "./context";

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
	const workflowInspector = createWorkflowInspectorAdapter();

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

		const driver = new ActorWorkflowDriver(actor, runCtx);

		const handle = runWorkflow(
			actor.id,
			async (ctx) => await fn(new ActorWorkflowContext(ctx, runCtx)),
			undefined,
			driver,
			{
				mode: "live",
				logger: runCtx.log,
				onHistoryUpdated: workflowInspector.update,
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
			runCtx.log.error({
				msg: "workflow run failed",
				error: stringifyError(error),
			});
			throw error;
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
		inspector: { workflow: workflowInspector.adapter },
	};

	return runWithConfig;
}
