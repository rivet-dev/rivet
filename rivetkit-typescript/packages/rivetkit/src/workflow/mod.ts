import { ACTOR_CONTEXT_INTERNAL_SYMBOL } from "@/actor/contexts/base/actor";
import type { RunContext } from "@/actor/contexts/run";
import type { AnyDatabaseProvider } from "@/actor/database";
import type { AnyActorInstance } from "@/actor/instance/mod";
import type { RunConfig } from "@/actor/config";
import { stringifyError } from "@/utils";
import { runWorkflow } from "@rivetkit/workflow-engine";
import invariant from "invariant";
import { ActorWorkflowContext } from "./context";
import { ActorWorkflowDriver, workflowQueueName } from "./driver";

export { Loop } from "@rivetkit/workflow-engine";
export { workflowQueueName } from "./driver";
export { ActorWorkflowContext } from "./context";

export function workflow<
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider,
>(
	fn: (
		ctx: ActorWorkflowContext<
			TState,
			TConnParams,
			TConnState,
			TVars,
			TInput,
			TDatabase
		>,
	) => Promise<unknown>,
): RunConfig {
	async function run(
		runCtx: RunContext<
			TState,
			TConnParams,
			TConnState,
			TVars,
			TInput,
			TDatabase
		>,
	): Promise<never> {
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
			{ mode: "live", logger: runCtx.log },
		);

		runCtx.abortSignal.addEventListener(
			"abort",
			() => {
				handle.evict();
			},
			{ once: true },
		);

		runCtx.waitUntil(
			handle.result
				.then(() => {
					// Ignore normal completion; the actor will be restarted if needed.
				})
				.catch((error) => {
					runCtx.log.error({
						msg: "workflow run failed",
						error: stringifyError(error),
					});
				}),
		);

		return await new Promise<never>(() => {
			// Intentionally never resolve to keep the run handler alive.
		});
	}

	return {
		icon: "diagram-project",
		run,
	};
}
