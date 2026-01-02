import { beforeEach, describe, expect, it } from "vitest";
import {
	InMemoryDriver,
	RollbackCheckpointError,
	runWorkflow,
	type WorkflowContextInterface,
} from "../src/testing.js";

const modes = ["yield", "live"] as const;

for (const mode of modes) {
	describe(`Workflow Engine Rollback (${mode})`, { sequential: true }, () => {
		let driver: InMemoryDriver;

		beforeEach(() => {
			driver = new InMemoryDriver();
			driver.latency = 0;
		});

		it("should execute rollback steps in reverse order", async () => {
			const rollbacks: string[] = [];

			const workflow = async (ctx: WorkflowContextInterface) => {
				await ctx.rollbackCheckpoint("checkpoint");
				await ctx.step({
					name: "first",
					run: async () => "one",
					rollback: async (_ctx, output) => {
						rollbacks.push(`first:${output}`);
					},
				});
				await ctx.step({
					name: "second",
					run: async () => "two",
					rollback: async (_ctx, output) => {
						rollbacks.push(`second:${output}`);
					},
				});
				throw new Error("boom");
			};

			await expect(
				runWorkflow("wf-1", workflow, undefined, driver, { mode })
					.result,
			).rejects.toThrow("boom");

			expect(rollbacks).toEqual(["second:two", "first:one"]);
		});

		it("should error if rollback checkpoint missing", async () => {
			const workflow = async (ctx: WorkflowContextInterface) => {
				await ctx.step({
					name: "missing-checkpoint",
					run: async () => "value",
					rollback: async () => {
						return;
					},
				});
			};

			await expect(
				runWorkflow("wf-1", workflow, undefined, driver, { mode })
					.result,
			).rejects.toThrow(RollbackCheckpointError);
		});

		it("should resume rollback after eviction", async () => {
			const rollbacks: string[] = [];
			let unblockRollback: (() => void) | undefined;
			let startRollback: (() => void) | undefined;

			const rollbackGate = new Promise<void>((resolve) => {
				unblockRollback = () => resolve();
			});
			const rollbackStarted = new Promise<void>((resolve) => {
				startRollback = () => resolve();
			});

			const workflow = async (ctx: WorkflowContextInterface) => {
				await ctx.rollbackCheckpoint("checkpoint");
				await ctx.step({
					name: "first",
					run: async () => "one",
					rollback: async () => {
						rollbacks.push("first");
					},
				});
				await ctx.step({
					name: "second",
					run: async () => "two",
					rollback: async (rollbackCtx) => {
						if (startRollback) {
							startRollback();
						}
						await rollbackGate;
						if (rollbackCtx.abortSignal.aborted) {
							return;
						}
						rollbacks.push("second");
					},
				});
				await ctx.step({
					name: "third",
					run: async () => "three",
					rollback: async () => {
						rollbacks.push("third");
					},
				});
				throw new Error("boom");
			};

			const handle = runWorkflow("wf-1", workflow, undefined, driver, {
				mode,
			});

			await rollbackStarted;
			handle.evict();

			const result = await handle.result;
			expect(result.state).toBe("rolling_back");
			expect(rollbacks).toEqual(["third"]);

			if (unblockRollback) {
				unblockRollback();
			}

			await expect(
				runWorkflow("wf-1", workflow, undefined, driver, { mode })
					.result,
			).rejects.toThrow("boom");

			expect(rollbacks).toEqual(["third", "second", "first"]);
		});
	});
}
