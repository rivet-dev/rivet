import { beforeEach, describe, expect, it } from "vitest";
import {
	CriticalError,
	EntryInProgressError,
	HistoryDivergedError,
	InMemoryDriver,
	RollbackError,
	runWorkflow,
	StepExhaustedError,
	type WorkflowContextInterface,
} from "../src/testing.js";
import { buildHistoryPrefixAll, keyStartsWith } from "../src/keys.js";
import { deserializeEntry, serializeEntry } from "../schemas/serde.js";

const modes = ["yield", "live"] as const;

class CountingDriver extends InMemoryDriver {
	batchCalls = 0;

	async batch(
		writes: { key: Uint8Array; value: Uint8Array }[],
	): Promise<void> {
		this.batchCalls += 1;
		await super.batch(writes);
	}
}

class StripStepHistoryErrorDriver extends InMemoryDriver {
	override async batch(
		writes: { key: Uint8Array; value: Uint8Array }[],
	): Promise<void> {
		const historyPrefix = buildHistoryPrefixAll();
		const rewritten = writes.map((write) => {
			if (!keyStartsWith(write.key, historyPrefix)) {
				return write;
			}

			const entry = deserializeEntry(write.value);
			if (entry.kind.type === "step") {
				// Simulate a driver/crash scenario where the step error is not persisted
				// to the history entry, even though retries/exhaustion metadata is.
				entry.kind.data.error = undefined;
				return { key: write.key, value: serializeEntry(entry) };
			}

			return write;
		});

		return await super.batch(rewritten);
	}
}

for (const mode of modes) {
	describe(`Workflow Engine Steps (${mode})`, { sequential: true }, () => {
		let driver: CountingDriver;

		beforeEach(() => {
			driver = new CountingDriver();
			driver.latency = 0;
		});

		it("should execute a simple step", async () => {
			const workflow = async (ctx: WorkflowContextInterface) => {
				const result = await ctx.step("my-step", async () => {
					return "hello world";
				});
				return result;
			};

			const result = await runWorkflow(
				"wf-1",
				workflow,
				undefined,
				driver,
				{ mode },
			).result;

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

			await runWorkflow("wf-1", workflow, undefined, driver, { mode })
				.result;
			expect(callCount).toBe(1);

			await runWorkflow("wf-1", workflow, undefined, driver, { mode })
				.result;
			expect(callCount).toBe(1);
		});

		it("should replay void step on restart", async () => {
			let callCount = 0;

			const workflow = async (ctx: WorkflowContextInterface) => {
				const result = await ctx.step("void-step", async () => {
					callCount++;
				});
				return result;
			};

			await runWorkflow("wf-1", workflow, undefined, driver, { mode })
				.result;
			expect(callCount).toBe(1);

			await runWorkflow("wf-1", workflow, undefined, driver, { mode })
				.result;
			expect(callCount).toBe(1);
		});

		it("should execute multiple steps in sequence", async () => {
			const workflow = async (ctx: WorkflowContextInterface) => {
				const a = await ctx.step("step-a", async () => 1);
				const b = await ctx.step("step-b", async () => 2);
				const c = await ctx.step("step-c", async () => 3);
				return a + b + c;
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

		it("should retry failed steps", async () => {
			let attempts = 0;

			const workflow = async (ctx: WorkflowContextInterface) => {
				return await ctx.step({
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
				});
			};

			try {
				await runWorkflow("wf-1", workflow, undefined, driver, { mode })
					.result;
			} catch {}

			try {
				await runWorkflow("wf-1", workflow, undefined, driver, { mode })
					.result;
			} catch {}

			const result = await runWorkflow(
				"wf-1",
				workflow,
				undefined,
				driver,
				{ mode },
			).result;

			expect(result.state).toBe("completed");
			expect(result.output).toBe("success");
			expect(attempts).toBe(3);
		});

		it("should yield during backoff retries", async () => {
			let attempts = 0;

			const workflow = async (ctx: WorkflowContextInterface) => {
				return await ctx.step({
					name: "always-fails",
					maxRetries: 3,
					retryBackoffBase: 50,
					retryBackoffMax: 100,
					run: async () => {
						attempts++;
						throw new Error("Failure");
					},
				});
			};

			if (mode === "yield") {
				const result1 = await runWorkflow(
					"wf-1",
					workflow,
					undefined,
					driver,
					{ mode },
				).result;
				expect(result1.state).toBe("sleeping");
				expect(result1.sleepUntil).toBeDefined();

				const result2 = await runWorkflow(
					"wf-1",
					workflow,
					undefined,
					driver,
					{ mode },
				).result;
				expect(result2.state).toBe("sleeping");
				expect(result2.sleepUntil).toBeDefined();
				expect(result2.sleepUntil).toBeGreaterThan(Date.now());
				expect(driver.getAlarm("wf-1")).toBe(result2.sleepUntil);
				expect(attempts).toBe(1);
				return;
			}

			await expect(
				runWorkflow("wf-1", workflow, undefined, driver, { mode })
					.result,
			).rejects.toThrow(StepExhaustedError);
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
				runWorkflow("wf-1", workflow, undefined, driver, { mode })
					.result,
			).rejects.toThrow(CriticalError);

			expect(attempts).toBe(1);
		});

		it("should not retry RollbackError", async () => {
			let attempts = 0;

			const workflow = async (ctx: WorkflowContextInterface) => {
				return await ctx.step("rollback-step", async () => {
					attempts++;
					throw new RollbackError("Rollback now");
				});
			};

			await expect(
				runWorkflow("wf-1", workflow, undefined, driver, { mode })
					.result,
			).rejects.toThrow(RollbackError);

			expect(attempts).toBe(1);
		});

		it("should exhaust retries", async () => {
			let attempts = 0;

			const workflow = async (ctx: WorkflowContextInterface) => {
				return await ctx.step({
					name: "always-fails",
					maxRetries: 2,
					retryBackoffBase: 1,
					run: async () => {
						attempts++;
						throw new Error("Always fails");
					},
				});
			};

			if (mode === "yield") {
				for (let i = 0; i < 3; i++) {
					try {
						await runWorkflow("wf-1", workflow, undefined, driver, {
							mode,
						}).result;
					} catch {}
				}

				await expect(
					runWorkflow("wf-1", workflow, undefined, driver, { mode })
						.result,
				).rejects.toThrow(StepExhaustedError);
				return;
			}

			await expect(
				runWorkflow("wf-1", workflow, undefined, driver, { mode })
					.result,
			).rejects.toThrow(StepExhaustedError);
		});

		it("should surface the last error even if step history is missing the error", async () => {
			const driver = new StripStepHistoryErrorDriver();
			driver.latency = 0;

			const workflow = async (ctx: WorkflowContextInterface) => {
				return await ctx.step({
					name: "always-fails",
					maxRetries: 1,
					retryBackoffBase: 0,
					retryBackoffMax: 0,
					run: async () => {
						throw new Error("Always fails");
					},
				});
			};

			if (mode === "yield") {
				const res1 = await runWorkflow(
					"wf-1",
					workflow,
					undefined,
					driver,
					{ mode },
				).result;
				expect(res1.state).toBe("sleeping");

				await expect(
					runWorkflow("wf-1", workflow, undefined, driver, { mode })
						.result,
				).rejects.toThrow(StepExhaustedError);
				await expect(
					runWorkflow("wf-1", workflow, undefined, driver, { mode })
						.result,
				).rejects.toThrow(/Always fails/);
				return;
			}

			await expect(
				runWorkflow("wf-1", workflow, undefined, driver, { mode }).result,
			).rejects.toThrow(StepExhaustedError);
			await expect(
				runWorkflow("wf-1", workflow, undefined, driver, { mode }).result,
			).rejects.toThrow(/Always fails/);
		});

		it("should recover exhausted retries", async () => {
			let attempts = 0;

			const workflow = async (ctx: WorkflowContextInterface) => {
				return await ctx.step({
					name: "recoverable",
					maxRetries: 1,
					retryBackoffBase: 1,
					run: async () => {
						attempts++;
						throw new Error("Always fails");
					},
				});
			};

			const runOnce = () =>
				runWorkflow("wf-1", workflow, undefined, driver, { mode });

			if (mode === "yield") {
				await runOnce().result;
			}

			const exhaustedHandle = runOnce();
			await expect(exhaustedHandle.result).rejects.toThrow(
				StepExhaustedError,
			);

			const attemptsAfterExhaust = attempts;
			await exhaustedHandle.recover();

			if (mode === "yield") {
				await runOnce().result;
			} else {
				await expect(runOnce().result).rejects.toThrow(
					StepExhaustedError,
				);
			}

			expect(attempts).toBeGreaterThan(attemptsAfterExhaust);
		});

		it("should fail steps that exceed timeout", async () => {
			const workflow = async (ctx: WorkflowContextInterface) => {
				return await ctx.step({
					name: "timeout-step",
					timeout: 5,
					run: async () => {
						await new Promise((resolve) => setTimeout(resolve, 25));
						return "late";
					},
				});
			};

			await expect(
				runWorkflow("wf-1", workflow, undefined, driver, { mode })
					.result,
			).rejects.toThrow(CriticalError);
		});

		it("should fail when a step is not awaited", async () => {
			const workflow = async (ctx: WorkflowContextInterface) => {
				const first = ctx.step("step-a", async () => "a");
				await ctx.step("step-b", async () => "b");
				return await first;
			};

			await expect(
				runWorkflow("wf-1", workflow, undefined, driver, { mode })
					.result,
			).rejects.toThrow(EntryInProgressError);
		});

		it("should reject duplicate entry names", async () => {
			const workflow = async (ctx: WorkflowContextInterface) => {
				await ctx.step("dup", async () => "first");
				await ctx.step("dup", async () => "second");
				return "done";
			};

			await expect(
				runWorkflow("wf-1", workflow, undefined, driver, { mode })
					.result,
			).rejects.toThrow(HistoryDivergedError);
		});

		it("should batch ephemeral steps until a durable flush", async () => {
			const workflow = async (ctx: WorkflowContextInterface) => {
				await ctx.step({
					name: "ephemeral-a",
					ephemeral: true,
					run: async () => "a",
				});
				await ctx.step({
					name: "ephemeral-b",
					ephemeral: true,
					run: async () => "b",
				});
				return await ctx.step("durable", async () => "done");
			};

			const result = await runWorkflow(
				"wf-1",
				workflow,
				undefined,
				driver,
				{ mode },
			).result;

			expect(result.state).toBe("completed");
			expect(result.output).toBe("done");
			expect(driver.batchCalls).toBe(2);
		});
	});
}
