import { beforeEach, describe, expect, it } from "vitest";
import {
	InMemoryDriver,
	RaceError,
	runWorkflow,
	type WorkflowContextInterface,
} from "../src/testing.js";

const modes = ["yield", "live"] as const;

for (const mode of modes) {
	describe(`Workflow Engine Race (${mode})`, { sequential: true }, () => {
		let driver: InMemoryDriver;

		beforeEach(() => {
			driver = new InMemoryDriver();
			driver.latency = 0;
		});

		it("should return first completed branch", async () => {
			const workflow = async (ctx: WorkflowContextInterface) => {
				return await ctx.race("race", [
					{
						name: "fast",
						run: async (ctx) => {
							return await ctx.step(
								"fast-step",
								async () => "fast",
							);
						},
					},
					{
						name: "slow",
						run: async (ctx) => {
							return await ctx.step(
								"slow-step",
								async () => "slow",
							);
						},
					},
				]);
			};

			const result = await runWorkflow(
				"wf-1",
				workflow,
				undefined,
				driver,
				{ mode },
			).result;

			expect(result.state).toBe("completed");
			expect(result.output?.winner).toBe("fast");
			expect(result.output?.value).toBe("fast");
		});

		it("should cancel losing branches", async () => {
			let aborted = false;

			const workflow = async (ctx: WorkflowContextInterface) => {
				return await ctx.race("cancel", [
					{
						name: "fast",
						run: async () => "winner",
					},
					{
						name: "slow",
						run: async (ctx) => {
							await new Promise<void>((resolve) => {
								if (ctx.abortSignal.aborted) {
									aborted = true;
									resolve();
									return;
								}
								ctx.abortSignal.addEventListener(
									"abort",
									() => {
										aborted = true;
										resolve();
									},
									{ once: true },
								);
							});
							return "aborted";
						},
					},
				]);
			};

			const result = await runWorkflow(
				"wf-1",
				workflow,
				undefined,
				driver,
				{ mode },
			).result;

			expect(result.state).toBe("completed");
			expect(aborted).toBe(true);
		});

		it("should surface errors when all branches fail", async () => {
			const workflow = async (ctx: WorkflowContextInterface) => {
				return await ctx.race("fail", [
					{
						name: "one",
						run: async () => {
							throw new Error("fail one");
						},
					},
					{
						name: "two",
						run: async () => {
							throw new Error("fail two");
						},
					},
				]);
			};

			await expect(
				runWorkflow("wf-1", workflow, undefined, driver, { mode })
					.result,
			).rejects.toThrow(RaceError);
		});

		it("should replay race winner", async () => {
			let runs = 0;

			const workflow = async (ctx: WorkflowContextInterface) => {
				const result = await ctx.race("replay", [
					{
						name: "first",
						run: async () => {
							runs += 1;
							return "winner";
						},
					},
					{
						name: "second",
						run: async () => "loser",
					},
				]);

				return result.value;
			};

			await runWorkflow("wf-1", workflow, undefined, driver, { mode })
				.result;
			await runWorkflow("wf-1", workflow, undefined, driver, { mode })
				.result;

			expect(runs).toBe(1);
		});
	});
}
