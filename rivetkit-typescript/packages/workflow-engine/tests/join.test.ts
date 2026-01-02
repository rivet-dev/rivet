import { beforeEach, describe, expect, it } from "vitest";
import {
	InMemoryDriver,
	JoinError,
	runWorkflow,
	type WorkflowContextInterface,
} from "../src/testing.js";

const modes = ["yield", "live"] as const;

for (const mode of modes) {
	describe(`Workflow Engine Join (${mode})`, { sequential: true }, () => {
		let driver: InMemoryDriver;

		beforeEach(() => {
			driver = new InMemoryDriver();
			driver.latency = 0;
		});

		it("should execute branches in parallel", async () => {
			const order: string[] = [];

			const workflow = async (ctx: WorkflowContextInterface) => {
				const results = await ctx.join("parallel", {
					a: {
						run: async (ctx) => {
							order.push("a-start");
							const val = await ctx.step("step-a", async () => 1);
							order.push("a-end");
							return val;
						},
					},
					b: {
						run: async (ctx) => {
							order.push("b-start");
							const val = await ctx.step("step-b", async () => 2);
							order.push("b-end");
							return val;
						},
					},
				});

				return results.a + results.b;
			};

			const result = await runWorkflow(
				"wf-1",
				workflow,
				undefined,
				driver,
				{ mode },
			).result;

			expect(result.state).toBe("completed");
			expect(result.output).toBe(3);
			expect(order.indexOf("a-start")).toBeLessThan(
				order.indexOf("b-end"),
			);
			expect(order.indexOf("b-start")).toBeLessThan(
				order.indexOf("a-end"),
			);
		});

		it("should wait for all branches even on error", async () => {
			let bCompleted = false;

			const workflow = async (ctx: WorkflowContextInterface) => {
				await ctx.join("parallel", {
					a: {
						run: async () => {
							throw new Error("A failed");
						},
					},
					b: {
						run: async (ctx) => {
							await ctx.step("step-b", async () => {
								bCompleted = true;
								return "b";
							});
							return "b";
						},
					},
				});
			};

			await expect(
				runWorkflow("wf-1", workflow, undefined, driver, { mode })
					.result,
			).rejects.toThrow();

			expect(bCompleted).toBe(true);
		});

		it("should surface join errors per branch", async () => {
			const workflow = async (ctx: WorkflowContextInterface) => {
				await ctx.join("parallel", {
					good: {
						run: async () => "ok",
					},
					bad: {
						run: async () => {
							throw new Error("boom");
						},
					},
				});
			};

			await expect(
				runWorkflow("wf-1", workflow, undefined, driver, { mode })
					.result,
			).rejects.toThrow(JoinError);

			try {
				await runWorkflow("wf-1", workflow, undefined, driver, { mode })
					.result;
			} catch (error) {
				const joinError = error as JoinError;
				expect(Object.keys(joinError.errors)).toEqual(["bad"]);
			}
		});

		it("should replay join results", async () => {
			let callCount = 0;

			const workflow = async (ctx: WorkflowContextInterface) => {
				const result = await ctx.join("replay", {
					one: {
						run: async () => {
							callCount += 1;
							return "one";
						},
					},
					two: {
						run: async () => "two",
					},
				});

				return result.one;
			};

			await runWorkflow("wf-1", workflow, undefined, driver, { mode })
				.result;
			await runWorkflow("wf-1", workflow, undefined, driver, { mode })
				.result;

			expect(callCount).toBe(1);
		});
	});
}
