import { beforeEach, describe, expect, it } from "vitest";
import {
	EvictedError,
	InMemoryDriver,
	runWorkflow,
	type WorkflowContextInterface,
} from "../src/testing.js";

const modes = ["yield", "live"] as const;

for (const mode of modes) {
	describe(
		`Workflow Engine Eviction and Cancellation (${mode})`,
		{ sequential: true },
		() => {
			let driver: InMemoryDriver;

			beforeEach(() => {
				driver = new InMemoryDriver();
				driver.latency = 0;
			});

			it("should surface eviction through the abort event", async () => {
				const workflow = async (ctx: WorkflowContextInterface) => {
					await new Promise<void>((resolve) => {
						if (ctx.abortSignal.aborted) {
							resolve();
							return;
						}
						ctx.abortSignal.addEventListener(
							"abort",
							() => resolve(),
							{
								once: true,
							},
						);
					});
					return ctx.isEvicted();
				};

				const handle = runWorkflow(
					"wf-1",
					workflow,
					undefined,
					driver,
					{ mode },
				);
				handle.evict();

				const result = await handle.result;
				expect(result.state).toBe("completed");
				expect(result.output).toBe(true);
			});

			it("should cancel workflow and clear alarms", async () => {
				const workflow = async (_ctx: WorkflowContextInterface) => {
					return "done";
				};

				const handle = runWorkflow(
					"wf-1",
					workflow,
					undefined,
					driver,
					{ mode },
				);
				await driver.setAlarm("wf-1", Date.now() + 1000);

				await handle.cancel();

				await expect(handle.result).rejects.toThrow(EvictedError);
				expect(await handle.getState()).toBe("cancelled");
				expect(driver.getAlarm("wf-1")).toBeUndefined();

				await expect(
					runWorkflow("wf-1", workflow, undefined, driver, { mode })
						.result,
				).rejects.toThrow(EvictedError);
			});
		},
	);
}
