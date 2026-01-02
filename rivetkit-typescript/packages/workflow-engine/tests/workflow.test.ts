import { describe, it, expect, beforeEach } from "vitest";
import {
	InMemoryDriver,
	runWorkflow,
	Loop,
	CriticalError,
	SleepError,
	StepExhaustedError,
	serializeSignal,
	buildSignalKey,
	generateId,
	type WorkflowContextInterface,
} from "../src/testing.js";

describe("Workflow Engine", { sequential: true }, () => {
	let driver: InMemoryDriver;

	beforeEach(() => {
		driver = new InMemoryDriver();
		driver.latency = 0; // Disable latency for faster tests
	});

	describe("Steps", () => {
		it("should execute a simple step", async () => {
			const workflow = async (ctx: WorkflowContextInterface) => {
				const result = await ctx.step("my-step", async () => {
					return "hello world";
				});
				return result;
			};

			const result = await runWorkflow("wf-1", workflow, undefined, driver).result;

			expect(result.state).toBe("completed");
			expect(result.output).toBe("hello world");
		});

		it("should replay step on restart", async () => {
			let callCount = 0;

			const workflow = async (ctx: WorkflowContextInterface) => {
				const result = await ctx.step("my-step", async () => {
					callCount++;
					return "hello";
				});
				return result;
			};

			// First run
			await runWorkflow("wf-1", workflow, undefined, driver).result;
			expect(callCount).toBe(1);

			// Second run - should replay
			await runWorkflow("wf-1", workflow, undefined, driver).result;
			expect(callCount).toBe(1); // Should not increment
		});

		it("should execute multiple steps in sequence", async () => {
			const workflow = async (ctx: WorkflowContextInterface) => {
				const a = await ctx.step("step-a", async () => 1);
				const b = await ctx.step("step-b", async () => 2);
				const c = await ctx.step("step-c", async () => 3);
				return a + b + c;
			};

			const result = await runWorkflow("wf-1", workflow, undefined, driver).result;

			expect(result.state).toBe("completed");
			expect(result.output).toBe(6);
		});

		it("should retry failed steps", async () => {
			let attempts = 0;

			const workflow = async (ctx: WorkflowContextInterface) => {
				return await ctx.step(
					{
						name: "flaky-step",
						maxRetries: 3,
						retryBackoffBase: 1,
						retryBackoffMax: 10,
						run: async () => {
							attempts++;
							if (attempts < 3) {
								throw new Error("Transient failure");
							}
							return "success";
						},
					},
				);
			};

			// First attempt fails
			try {
				await runWorkflow("wf-1", workflow, undefined, driver).result;
			} catch {}

			// Second attempt fails
			try {
				await runWorkflow("wf-1", workflow, undefined, driver).result;
			} catch {}

			// Third attempt succeeds
			const result = await runWorkflow("wf-1", workflow, undefined, driver).result;

			expect(result.state).toBe("completed");
			expect(result.output).toBe("success");
			expect(attempts).toBe(3);
		});

		it("should not retry CriticalError", async () => {
			let attempts = 0;

			const workflow = async (ctx: WorkflowContextInterface) => {
				return await ctx.step("critical-step", async () => {
					attempts++;
					throw new CriticalError("Unrecoverable");
				});
			};

			await expect(
				runWorkflow("wf-1", workflow, undefined, driver).result,
			).rejects.toThrow(CriticalError);

			// Should not retry
			expect(attempts).toBe(1);
		});

		it("should exhaust retries", async () => {
			let attempts = 0;

			const workflow = async (ctx: WorkflowContextInterface) => {
				return await ctx.step(
					{
						name: "always-fails",
						maxRetries: 2,
						retryBackoffBase: 1,
						run: async () => {
							attempts++;
							throw new Error("Always fails");
						},
					},
				);
			};

			// Exhaust retries
			for (let i = 0; i < 3; i++) {
				try {
					await runWorkflow("wf-1", workflow, undefined, driver).result;
				} catch {}
			}

			// Should throw StepExhaustedError
			await expect(
				runWorkflow("wf-1", workflow, undefined, driver).result,
			).rejects.toThrow(StepExhaustedError);
		});
	});

	describe("Loops", () => {
		it("should execute a simple loop", async () => {
			const workflow = async (ctx: WorkflowContextInterface) => {
				return await ctx.loop(
					{
						name: "count-loop",
						state: { count: 0 },
						run: async (ctx, state) => {
							if (state.count >= 3) {
								return Loop.break(state.count);
							}
							await ctx.step(`step-${state.count}`, async () => {});
							return Loop.continue({ count: state.count + 1 });
						},
					},
				);
			};

			const result = await runWorkflow("wf-1", workflow, undefined, driver).result;

			expect(result.state).toBe("completed");
			expect(result.output).toBe(3);
		});

		it("should resume loop from saved state", async () => {
			let iteration = 0;

			const workflow = async (ctx: WorkflowContextInterface) => {
				return await ctx.loop(
					{
						name: "resume-loop",
						state: { count: 0 },
						commitInterval: 2, // Commit every 2 iterations
						run: async (ctx, state) => {
							iteration++;

							if (state.count >= 5) {
								return Loop.break(state.count);
							}

							// Simulate crash on iteration 3
							if (state.count === 2 && iteration === 3) {
								throw new Error("Simulated crash");
							}

							return Loop.continue({ count: state.count + 1 });
						},
					},
				);
			};

			// First run - crashes on iteration 3
			try {
				await runWorkflow("wf-1", workflow, undefined, driver).result;
			} catch {}

			// Reset iteration counter to see how many new iterations run
			iteration = 0;

			// Second run - should resume from last committed state
			const result = await runWorkflow("wf-1", workflow, undefined, driver).result;

			expect(result.state).toBe("completed");
			expect(result.output).toBe(5);
		});
	});

	describe("Sleep", () => {
		it("should yield on long sleep", async () => {
			const workflow = async (ctx: WorkflowContextInterface) => {
				await ctx.sleep("my-sleep", 10000);
				return "done";
			};

			const result = await runWorkflow("wf-1", workflow, undefined, driver).result;

			expect(result.state).toBe("sleeping");
			expect(result.sleepUntil).toBeDefined();
			expect(result.sleepUntil).toBeGreaterThan(Date.now());
		});

		it("should complete short sleep in memory", async () => {
			driver.workerPollInterval = 1000; // High threshold

			const workflow = async (ctx: WorkflowContextInterface) => {
				await ctx.sleep("short-sleep", 10); // 10ms
				return "done";
			};

			const result = await runWorkflow("wf-1", workflow, undefined, driver).result;

			expect(result.state).toBe("completed");
			expect(result.output).toBe("done");
		});

		it("should resume after sleep deadline", async () => {
			const workflow = async (ctx: WorkflowContextInterface) => {
				await ctx.sleep("my-sleep", 1); // Very short sleep
				return "done";
			};

			// First run - starts sleeping
			const result1 = await runWorkflow("wf-1", workflow, undefined, driver).result;

			// Wait for deadline
			await new Promise((r) => setTimeout(r, 10));

			// Second run - should complete
			const result2 = await runWorkflow("wf-1", workflow, undefined, driver).result;

			expect(result2.state).toBe("completed");
			expect(result2.output).toBe("done");
		});
	});

	describe("Join", () => {
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

			const result = await runWorkflow("wf-1", workflow, undefined, driver).result;

			expect(result.state).toBe("completed");
			expect(result.output).toBe(3);

			// Both branches should start before either ends (parallel)
			expect(order.indexOf("a-start")).toBeLessThan(order.indexOf("b-end"));
			expect(order.indexOf("b-start")).toBeLessThan(order.indexOf("a-end"));
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
				runWorkflow("wf-1", workflow, undefined, driver).result,
			).rejects.toThrow();

			expect(bCompleted).toBe(true);
		});
	});

	describe("Race", () => {
		it("should return first completed branch", async () => {
			const workflow = async (ctx: WorkflowContextInterface) => {
				return await ctx.race("race", [
					{
						name: "fast",
						run: async (ctx) => {
							return await ctx.step("fast-step", async () => "fast");
						},
					},
					{
						name: "slow",
						run: async (ctx) => {
							// This would sleep but fast completes first
							return await ctx.step("slow-step", async () => "slow");
						},
					},
				]);
			};

			const result = await runWorkflow("wf-1", workflow, undefined, driver).result;

			expect(result.state).toBe("completed");
			expect(result.output?.winner).toBe("fast");
			expect(result.output?.value).toBe("fast");
		});
	});

	describe("Signals", () => {
		it("should wait for signals", async () => {
			const workflow = async (ctx: WorkflowContextInterface) => {
				const signal = await ctx.listen<string>("wait-signal", "my-signal");
				return signal;
			};

			// First run - should wait
			const result1 = await runWorkflow("wf-1", workflow, undefined, driver).result;
			expect(result1.state).toBe("sleeping");
			expect(result1.waitingForSignals).toContain("my-signal");
		});

		it("should consume pending signals", async () => {
			// Pre-add a signal using BARE serialization with binary key
			const signalId = generateId();
			await driver.set(
				buildSignalKey(signalId),
				serializeSignal({
					id: signalId,
					name: "my-signal",
					data: "hello",
					sentAt: Date.now(),
				}),
			);

			const workflow = async (ctx: WorkflowContextInterface) => {
				const signal = await ctx.listen<string>("wait-signal", "my-signal");
				return signal;
			};

			const result = await runWorkflow("wf-1", workflow, undefined, driver).result;

			expect(result.state).toBe("completed");
			expect(result.output).toBe("hello");
		});
	});

	describe("Removed", () => {
		it("should skip removed steps", async () => {
			// First, create a workflow with a step
			const workflow1 = async (ctx: WorkflowContextInterface) => {
				await ctx.step("old-step", async () => "old");
				return "done";
			};

			await runWorkflow("wf-1", workflow1, undefined, driver).result;

			// Now "update" the workflow to remove the step
			const workflow2 = async (ctx: WorkflowContextInterface) => {
				await ctx.removed("old-step", "step");
				return "updated";
			};

			const result = await runWorkflow("wf-1", workflow2, undefined, driver).result;

			expect(result.state).toBe("completed");
			expect(result.output).toBe("updated");
		});
	});
});
