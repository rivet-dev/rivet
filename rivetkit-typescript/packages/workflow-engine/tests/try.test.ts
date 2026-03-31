import { beforeEach, describe, expect, it } from "vitest";
import {
	CriticalError,
	InMemoryDriver,
	RollbackError,
	runWorkflow,
	type WorkflowContextInterface,
} from "../src/testing.js";

const modes = ["yield", "live"] as const;

for (const mode of modes) {
	describe(`Workflow Engine Try (${mode})`, { sequential: true }, () => {
		let driver: InMemoryDriver;

		beforeEach(() => {
			driver = new InMemoryDriver();
			driver.latency = 0;
		});

		it("should return handled critical failures from tryStep", async () => {
			const workflow = async (ctx: WorkflowContextInterface) => {
				return await ctx.tryStep("critical-step", async () => {
					throw new CriticalError("stop");
				});
			};

			const result = await runWorkflow(
				"wf-try-step-critical",
				workflow,
				undefined,
				driver,
				{ mode },
			).result;

			expect(result.state).toBe("completed");
			expect(result.output).toMatchObject({
				ok: false,
				failure: {
					kind: "critical",
					stepName: "critical-step",
					attempts: 1,
					error: {
						name: "CriticalError",
						message: "stop",
					},
				},
			});
		});

		it("should preserve retries before returning exhausted tryStep failures", async () => {
			let attempts = 0;

			const workflow = async (ctx: WorkflowContextInterface) => {
				return await ctx.tryStep({
					name: "flaky-step",
					maxRetries: 1,
					retryBackoffBase: 5,
					retryBackoffMax: 5,
					run: async () => {
						attempts += 1;
						throw new Error("boom");
					},
				});
			};

			const firstResult = await runWorkflow(
				"wf-try-step-exhausted",
				workflow,
				undefined,
				driver,
				{ mode },
			).result;

			if (mode === "yield") {
				expect(firstResult.state).toBe("sleeping");
				await new Promise((resolve) => setTimeout(resolve, 10));

				const secondResult = await runWorkflow(
					"wf-try-step-exhausted",
					workflow,
					undefined,
					driver,
					{ mode },
				).result;

				expect(secondResult.state).toBe("completed");
				expect(secondResult.output).toMatchObject({
					ok: false,
					failure: {
						kind: "exhausted",
						stepName: "flaky-step",
						attempts: 2,
						error: {
							message: "boom",
						},
					},
				});
			} else {
				expect(firstResult.state).toBe("completed");
				expect(firstResult.output).toMatchObject({
					ok: false,
					failure: {
						kind: "exhausted",
						stepName: "flaky-step",
						attempts: 2,
						error: {
							message: "boom",
						},
					},
				});
			}

			expect(attempts).toBe(2);
		});

		it("should not catch rollback in tryStep by default", async () => {
			const workflow = async (ctx: WorkflowContextInterface) => {
				return await ctx.tryStep("rollback-step", async () => {
					throw new RollbackError("rollback");
				});
			};

			await expect(
				runWorkflow(
					"wf-try-step-rollback-default",
					workflow,
					undefined,
					driver,
					{ mode },
				).result,
			).rejects.toThrow(RollbackError);
		});

		it("should catch rollback in tryStep when configured", async () => {
			const workflow = async (ctx: WorkflowContextInterface) => {
				return await ctx.tryStep({
					name: "rollback-step",
					catch: ["rollback"],
					run: async () => {
						throw new RollbackError("rollback");
					},
				});
			};

			const result = await runWorkflow(
				"wf-try-step-rollback-caught",
				workflow,
				undefined,
				driver,
				{ mode },
			).result;

			expect(result.state).toBe("completed");
			expect(result.output).toMatchObject({
				ok: false,
				failure: {
					kind: "rollback",
					stepName: "rollback-step",
					attempts: 1,
					error: {
						name: "RollbackError",
						message: "rollback",
					},
				},
			});
		});

		it("should catch terminal step failures inside try blocks", async () => {
			const workflow = async (ctx: WorkflowContextInterface) => {
				return await ctx.try("payment-flow", async (blockCtx) => {
					return await blockCtx.step({
						name: "charge",
						maxRetries: 0,
						run: async () => {
							throw new CriticalError("declined");
						},
					});
				});
			};

			const result = await runWorkflow(
				"wf-try-block-step",
				workflow,
				undefined,
				driver,
				{ mode },
			).result;

			expect(result.state).toBe("completed");
			expect(result.output).toMatchObject({
				ok: false,
				failure: {
					source: "step",
					name: "charge",
					error: {
						name: "CriticalError",
						message: "declined",
					},
					step: {
						kind: "critical",
						stepName: "charge",
						attempts: 1,
						error: {
							name: "CriticalError",
							message: "declined",
						},
					},
				},
			});
		});

		it("should catch join failures inside try blocks", async () => {
			const workflow = async (ctx: WorkflowContextInterface) => {
				return await ctx.try("parallel-flow", async (blockCtx) => {
					return await blockCtx.join("parallel", {
						good: {
							run: async () => "ok",
						},
						bad: {
							run: async () => {
								throw new Error("boom");
							},
						},
					});
				});
			};

			const result = await runWorkflow(
				"wf-try-block-join",
				workflow,
				undefined,
				driver,
				{ mode },
			).result;

			expect(result.state).toBe("completed");
			expect(result.output).toMatchObject({
				ok: false,
				failure: {
					source: "join",
					name: "parallel",
					error: {
						name: "JoinError",
					},
				},
			});
		});

		it("should catch race failures inside try blocks", async () => {
			const workflow = async (ctx: WorkflowContextInterface) => {
				return await ctx.try("race-flow", async (blockCtx) => {
					return await blockCtx.race("contest", [
						{
							name: "one",
							run: async () => {
								throw new Error("one failed");
							},
						},
						{
							name: "two",
							run: async () => {
								throw new Error("two failed");
							},
						},
					]);
				});
			};

			const result = await runWorkflow(
				"wf-try-block-race",
				workflow,
				undefined,
				driver,
				{ mode },
			).result;

			expect(result.state).toBe("completed");
			expect(result.output).toMatchObject({
				ok: false,
				failure: {
					source: "race",
					name: "contest",
					error: {
						name: "RaceError",
					},
				},
			});
		});

		it("should not catch rollback in try blocks by default", async () => {
			const workflow = async (ctx: WorkflowContextInterface) => {
				return await ctx.try("rollback-flow", async () => {
					throw new RollbackError("rollback");
				});
			};

			await expect(
				runWorkflow(
					"wf-try-block-rollback-default",
					workflow,
					undefined,
					driver,
					{ mode },
				).result,
			).rejects.toThrow(RollbackError);
		});

		it("should catch direct rollback in try blocks when configured", async () => {
			const workflow = async (ctx: WorkflowContextInterface) => {
				return await ctx.try({
					name: "rollback-flow",
					catch: ["rollback"],
					run: async () => {
						throw new RollbackError("rollback");
					},
				});
			};

			const result = await runWorkflow(
				"wf-try-block-rollback-caught",
				workflow,
				undefined,
				driver,
				{ mode },
			).result;

			expect(result.state).toBe("completed");
			expect(result.output).toMatchObject({
				ok: false,
				failure: {
					source: "block",
					name: "rollback-flow",
					error: {
						name: "RollbackError",
						message: "rollback",
					},
				},
			});
		});
	});
}
