import { beforeEach, describe, expect, it } from "vitest";
import {
	InMemoryDriver,
	runWorkflow,
	type WorkflowContextInterface,
} from "../src/testing.js";

const modes = ["yield", "live"] as const;

for (const mode of modes) {
	describe(`Workflow Engine Handle (${mode})`, { sequential: true }, () => {
		let driver: InMemoryDriver;

		beforeEach(() => {
			driver = new InMemoryDriver();
			driver.latency = 0;
		});

		it("should send messages via handle", async () => {
			const workflow = async (ctx: WorkflowContextInterface) => {
				const message = await ctx.queue.next<string>("wait", {
					names: ["message-name"],
				});
				return message.body;
			};

			const handle = runWorkflow("wf-1", workflow, undefined, driver, {
				mode,
			});

			if (mode === "yield") {
				await handle.result;
				await handle.message("message-name", "payload");

				const result = await runWorkflow(
					"wf-1",
					workflow,
					undefined,
					driver,
					{ mode },
				).result;

				expect(result.state).toBe("completed");
				expect(result.output).toBe("payload");
				return;
			}

			await handle.message("message-name", "payload");
			const result = await handle.result;

			expect(result.state).toBe("completed");
			expect(result.output).toBe("payload");
		});

		it("should set alarms with wake", async () => {
			const workflow = async (_ctx: WorkflowContextInterface) => {
				return "done";
			};

			const handle = runWorkflow("wf-1", workflow, undefined, driver, {
				mode,
			});
			await handle.wake();

			const alarm = driver.getAlarm("wf-1");
			if (mode === "yield") {
				expect(alarm).toBeDefined();
				expect(alarm).toBeLessThanOrEqual(Date.now());
			} else {
				expect(alarm).toBeUndefined();
			}
		});

		it("should read output and state", async () => {
			const workflow = async (_ctx: WorkflowContextInterface) => {
				return "finished";
			};

			const handle = runWorkflow("wf-1", workflow, undefined, driver, {
				mode,
			});
			const result = await handle.result;

			expect(result.state).toBe("completed");
			expect(await handle.getOutput()).toBe("finished");
			expect(await handle.getState()).toBe("completed");
		});
	});
}
