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

		it("should support nested joins inside branches", async () => {
			const workflow = async (ctx: WorkflowContextInterface) => {
				const results = await ctx.join("outer", {
					nested: {
						run: async (ctx) => {
							const inner = await ctx.join("inner", {
								left: {
									run: async (ctx) =>
										await ctx.step(
											"left-step",
											async () => 1,
										),
								},
								right: {
									run: async (ctx) =>
										await ctx.step(
											"right-step",
											async () => 2,
										),
								},
							});

							return inner.left + inner.right;
						},
					},
					plain: {
						run: async (ctx) =>
							await ctx.step("plain-step", async () => 3),
					},
				});

				return results.nested + results.plain;
			};

			const result = await runWorkflow(
				"wf-1",
				workflow,
				undefined,
				driver,
				{ mode },
			).result;

			expect(result.state).toBe("completed");
			expect(result.output).toBe(6);
		});

		it("should support nested races inside join branches", async () => {
			const workflow = async (ctx: WorkflowContextInterface) => {
				const results = await ctx.join("outer", {
					raced: {
						run: async (ctx) => {
							const nested = await ctx.race("inner-race", [
								{
									name: "fast",
									run: async (ctx) =>
										await ctx.step(
											"fast-step",
											async () => "winner",
										),
								},
								{
									name: "slow",
									run: async (ctx) => {
										await new Promise<void>((resolve) => {
											if (ctx.abortSignal.aborted) {
												resolve();
												return;
											}
											ctx.abortSignal.addEventListener(
												"abort",
												() => resolve(),
												{ once: true },
											);
										});
										return "loser";
									},
								},
							]);

							return nested.value;
						},
					},
					plain: {
						run: async (ctx) =>
							await ctx.step("plain-step", async () => "plain"),
					},
				});

				return `${results.raced}:${results.plain}`;
			};

			const result = await runWorkflow(
				"wf-1",
				workflow,
				undefined,
				driver,
				{ mode },
			).result;

			expect(result.state).toBe("completed");
			expect(result.output).toBe("winner:plain");
		});

		it("should preserve branch retries instead of failing the join", async () => {
			let attempts = 0;

			const workflow = async (ctx: WorkflowContextInterface) => {
				const result = await ctx.join("retrying-join", {
					flaky: {
						run: async (branchCtx) => {
							return await branchCtx.step({
								name: "flaky-step",
								maxRetries: 1,
								retryBackoffBase: 5,
								retryBackoffMax: 5,
								run: async () => {
									attempts += 1;
									if (attempts === 1) {
										throw new Error("retry");
									}
									return "a";
								},
							});
						},
					},
					stable: {
						run: async () => "b",
					},
				});

				return result.flaky + result.stable;
			};

			const firstResult = await runWorkflow(
				"wf-join-retry",
				workflow,
				undefined,
				driver,
				{ mode },
			).result;

			if (mode === "yield") {
				expect(firstResult.state).toBe("sleeping");
				await new Promise((resolve) => setTimeout(resolve, 10));

				const secondResult = await runWorkflow(
					"wf-join-retry",
					workflow,
					undefined,
					driver,
					{ mode },
				).result;

				expect(secondResult.state).toBe("completed");
				expect(secondResult.output).toBe("ab");
			} else {
				expect(firstResult.state).toBe("completed");
				expect(firstResult.output).toBe("ab");
			}

			expect(attempts).toBe(2);
		});
	});
}
