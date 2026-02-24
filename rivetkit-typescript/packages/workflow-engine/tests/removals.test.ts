import { beforeEach, describe, expect, it } from "vitest";
import {
	InMemoryDriver,
	Loop,
	runWorkflow,
	type WorkflowContextInterface,
} from "../src/testing.js";

const modes = ["yield", "live"] as const;

for (const mode of modes) {
	describe(
		`Workflow Engine Removed Entries (${mode})`,
		{ sequential: true },
		() => {
			let driver: InMemoryDriver;

			beforeEach(() => {
				driver = new InMemoryDriver();
				driver.latency = 0;
			});

			it("should skip removed entries of all kinds", async () => {
				const workflow1 = async (ctx: WorkflowContextInterface) => {
					await ctx.step("old-step", async () => "old");
					await ctx.loop({
						name: "old-loop",
						state: { count: 0 },
						run: async (_ctx, state) => {
							if (state.count >= 1) {
								return Loop.break("done");
							}
							return Loop.continue({ count: state.count + 1 });
						},
					});
					await ctx.queue.send("old-message", "message-data");
					await ctx.sleep("old-sleep", 0);
					await ctx.queue.next<string>("old-listen", {
						names: ["old-message"],
					});
					await ctx.join("old-join", {
						branch: {
							run: async () => "ok",
						},
					});
					await ctx.race("old-race", [
						{
							name: "fast",
							run: async () => "fast",
						},
					]);
					return "done";
				};

				await runWorkflow("wf-1", workflow1, undefined, driver, {
					mode,
				}).result;

				const workflow2 = async (ctx: WorkflowContextInterface) => {
					await ctx.removed("old-step", "step");
					await ctx.removed("old-loop", "loop");
					await ctx.removed("old-sleep", "sleep");
					await ctx.removed("old-listen", "message");
					await ctx.removed("old-join", "join");
					await ctx.removed("old-race", "race");
					return "updated";
				};

				const result = await runWorkflow(
					"wf-1",
					workflow2,
					undefined,
					driver,
					{ mode },
				).result;

				expect(result.state).toBe("completed");
				expect(result.output).toBe("updated");
			});
		},
	);
}
