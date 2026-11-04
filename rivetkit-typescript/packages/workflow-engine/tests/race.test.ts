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

		it("should support nested joins in the winning branch", async () => {
			const workflow = async (ctx: WorkflowContextInterface) => {
				return await ctx.race("outer-race", [
					{
						name: "fast",
						run: async (ctx) => {
							const nested = await ctx.join("inner-join", {
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

							return nested.left + nested.right;
						},
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
							return 0;
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
			expect(result.output).toEqual({
				winner: "fast",
				value: 3,
			});
		});

		it("should support nested races in the winning branch", async () => {
			const workflow = async (ctx: WorkflowContextInterface) => {
				return await ctx.race("outer-race", [
					{
						name: "fast",
						run: async (ctx) => {
							const nested = await ctx.race("inner-race", [
								{
									name: "nested-fast",
									run: async (ctx) =>
										await ctx.step(
											"nested-fast-step",
											async () => "nested-fast",
										),
								},
								{
									name: "nested-slow",
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
										return "nested-slow";
									},
								},
							]);

							return nested.value;
						},
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
							return "slow";
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
			expect(result.output).toEqual({
				winner: "fast",
				value: "nested-fast",
			});
		});

		it("should preserve branch retries instead of failing the race", async () => {
			let attempts = 0;

			const workflow = async (ctx: WorkflowContextInterface) => {
				return await ctx.race("retrying-race", [
					{
						name: "flaky",
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
									return "winner";
								},
							});
						},
					},
					{
						name: "slow",
						run: async (branchCtx) => {
							await branchCtx.sleep("slow-wait", 200);
							return "slow";
						},
					},
				]);
			};

			const firstResult = await runWorkflow(
				"wf-race-retry",
				workflow,
				undefined,
				driver,
				{ mode },
			).result;

			if (mode === "yield") {
				expect(firstResult.state).toBe("sleeping");
				await new Promise((resolve) => setTimeout(resolve, 10));

				const secondResult = await runWorkflow(
					"wf-race-retry",
					workflow,
					undefined,
					driver,
					{ mode },
				).result;

				expect(secondResult.state).toBe("completed");
				expect(secondResult.output).toEqual({
					winner: "flaky",
					value: "winner",
				});
			} else {
				expect(firstResult.state).toBe("completed");
				expect(firstResult.output).toEqual({
					winner: "flaky",
					value: "winner",
				});
			}

			expect(attempts).toBe(2);
		});

		it("should allow a sleep branch to time out slow in-process work and abort it", async () => {
			let workAborted = false;

			const workflow = async (ctx: WorkflowContextInterface) => {
				return await ctx.race<
					string | { status: string; duration: number }
				>("timeout-race", [
					{
						name: "work",
						run: async (ctx) => {
							return await ctx.step("slow-step", async () => {
								await new Promise<void>((resolve) => {
									const timer = setTimeout(resolve, 600);
									if (ctx.abortSignal.aborted) {
										workAborted = true;
										clearTimeout(timer);
										resolve();
										return;
									}
									ctx.abortSignal.addEventListener(
										"abort",
										() => {
											workAborted = true;
											clearTimeout(timer);
											resolve();
										},
										{ once: true },
									);
								});
								return "work-done";
							});
						},
					},
					{
						name: "timeout",
						run: async (ctx) => {
							await ctx.sleep("wait", 150);
							return { status: "timed-out", duration: 150 };
						},
					},
				]);
			};

			const result = await runWorkflow(
				"wf-race-timeout",
				workflow,
				undefined,
				driver,
				{ mode },
			).result;

			expect(result.state).toBe("completed");
			expect(result.output?.winner).toBe("timeout");
			expect(result.output?.value).toEqual({
				status: "timed-out",
				duration: 150,
			});
			expect(workAborted).toBe(true);
		});

		it("should pick earliest sleep branch among multiple sleeps when competing against slow work", async () => {
			const workflow = async (ctx: WorkflowContextInterface) => {
				return await ctx.race("multi-timeout-race", [
					{
						name: "work",
						run: async (ctx) => {
							return await ctx.step("slow-step", async () => {
								await new Promise((resolve) =>
									setTimeout(resolve, 800),
								);
								return "work-done";
							});
						},
					},
					{
						name: "slow-timeout",
						run: async (ctx) => {
							await ctx.sleep("wait-slow", 400);
							return "slow-timeout-winner";
						},
					},
					{
						name: "fast-timeout",
						run: async (ctx) => {
							await ctx.sleep("wait-fast", 150);
							return "fast-timeout-winner";
						},
					},
				]);
			};

			const result = await runWorkflow(
				"wf-race-multi-timeout",
				workflow,
				undefined,
				driver,
				{ mode },
			).result;

			expect(result.state).toBe("completed");
			expect(result.output?.winner).toBe("fast-timeout");
			expect(result.output?.value).toBe("fast-timeout-winner");
		});

		it("should allow a fast work branch to win before the sleep deadline", async () => {
			const workflow = async (ctx: WorkflowContextInterface) => {
				return await ctx.race("fast-work-race", [
					{
						name: "work",
						run: async (ctx) => {
							return await ctx.step("fast-step", async () => {
								return "fast-done";
							});
						},
					},
					{
						name: "timeout",
						run: async (ctx) => {
							await ctx.sleep("wait", 500);
							return null;
						},
					},
				]);
			};

			const result = await runWorkflow(
				"wf-race-fast-work",
				workflow,
				undefined,
				driver,
				{ mode },
			).result;

			expect(result.state).toBe("completed");
			expect(result.output?.winner).toBe("work");
			expect(result.output?.value).toBe("fast-done");
		});

		it("should yield to scheduler when all branches sleep without waiting in memory", async () => {
			const workflow = async (ctx: WorkflowContextInterface) => {
				return await ctx.race("all-sleep-race", [
					{
						name: "sleep-long",
						run: async (ctx) => {
							await ctx.sleep("wait-long", 5000);
							return "long";
						},
					},
					{
						name: "sleep-short",
						run: async (ctx) => {
							await ctx.sleep("wait-short", 3000);
							return "short";
						},
					},
				]);
			};

			const startTime = Date.now();
			const firstResult = await runWorkflow(
				"wf-race-all-sleep",
				workflow,
				undefined,
				driver,
				{ mode },
			).result;
			const elapsed = Date.now() - startTime;

			if (mode === "yield") {
				expect(firstResult.state).toBe("sleeping");
				// Should yield almost immediately, not wait 3000ms
				expect(elapsed).toBeLessThan(1000);
			} else {
				// In live mode, it will wait for the shorter sleep (3000ms) and complete
				expect(firstResult.state).toBe("completed");
				expect(firstResult.output?.winner).toBe("sleep-short");
				expect(firstResult.output?.value).toBe("short");
			}
		});
	});
}
