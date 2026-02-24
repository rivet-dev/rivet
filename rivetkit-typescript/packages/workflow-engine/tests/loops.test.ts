import { beforeEach, describe, expect, it } from "vitest";
import {
	InMemoryDriver,
	Loop,
	loadStorage,
	runWorkflow,
	type WorkflowContextInterface,
} from "../src/testing.js";

const modes = ["yield", "live"] as const;

function isLoopIteration(segment: unknown): segment is { iteration: number } {
	return (
		typeof segment === "object" &&
		segment !== null &&
		"iteration" in segment
	);
}

for (const mode of modes) {
	describe(`Workflow Engine Loops (${mode})`, { sequential: true }, () => {
		let driver: InMemoryDriver;

		beforeEach(() => {
			driver = new InMemoryDriver();
			driver.latency = 0;
		});

		it("should execute a simple loop", async () => {
			const workflow = async (ctx: WorkflowContextInterface) => {
				return await ctx.loop({
					name: "count-loop",
					state: { count: 0 },
					run: async (ctx, state) => {
						if (state.count >= 3) {
							return Loop.break(state.count);
						}
						await ctx.step(`step-${state.count}`, async () => {});
						return Loop.continue({ count: state.count + 1 });
					},
				});
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
		});

		it("should run a stateless loop", async () => {
			let iteration = 0;

			const workflow = async (ctx: WorkflowContextInterface) => {
				return await ctx.loop("stateless", async () => {
					iteration += 1;
					if (iteration >= 3) {
						return Loop.break("done");
					}
					return Loop.continue(undefined);
				});
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
		});

		it("should treat undefined return as continue in stateless loops", async () => {
			let iteration = 0;

			const workflow = async (ctx: WorkflowContextInterface) => {
				return await ctx.loop("stateless-implicit-continue", async () => {
					iteration += 1;
					if (iteration >= 3) {
						return Loop.break("done");
					}
					return undefined;
				});
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
		});

		it("should resume loop from saved state", async () => {
			let iteration = 0;

			const workflow = async (ctx: WorkflowContextInterface) => {
				return await ctx.loop({
					name: "resume-loop",
					state: { count: 0 },
					commitInterval: 2,
					run: async (_ctx, state) => {
						iteration++;

						if (state.count >= 5) {
							return Loop.break(state.count);
						}

						if (state.count === 2 && iteration === 3) {
							throw new Error("Simulated crash");
						}

						return Loop.continue({ count: state.count + 1 });
					},
				});
			};

			try {
				await runWorkflow("wf-1", workflow, undefined, driver, { mode })
					.result;
			} catch {}

			iteration = 0;

			const result = await runWorkflow(
				"wf-1",
				workflow,
				undefined,
				driver,
				{ mode },
			).result;

			expect(result.state).toBe("completed");
			expect(result.output).toBe(5);
		});

		it("should forget old iterations on history window", async () => {
			const workflow = async (ctx: WorkflowContextInterface) => {
				return await ctx.loop({
					name: "cleanup-loop",
					state: { count: 0 },
					commitInterval: 2,
					historyEvery: 2,
					historyKeep: 2,
					run: async (ctx, state) => {
						await ctx.step(`step-${state.count}`, async () => {});
						if (state.count >= 4) {
							return Loop.break(state.count);
						}
						return Loop.continue({ count: state.count + 1 });
					},
				});
			};

			await runWorkflow("wf-1", workflow, undefined, driver, { mode })
				.result;

			const storage = await loadStorage(driver);
			const iterations = [...storage.history.entries.values()]
				.flatMap((entry) => entry.location)
				.flatMap((segment) =>
					isLoopIteration(segment) ? [segment] : [],
				)
				.map((segment) => segment.iteration);

			const minIteration = Math.min(...iterations);
			expect(minIteration).toBeGreaterThanOrEqual(2);
		});

		it("should propagate loop errors", async () => {
			const workflow = async (ctx: WorkflowContextInterface) => {
				return await ctx.loop({
					name: "error-loop",
					state: { count: 0 },
					run: async () => {
						throw new Error("loop failure");
					},
				});
			};

			await expect(
				runWorkflow("wf-1", workflow, undefined, driver, { mode })
					.result,
			).rejects.toThrow("loop failure");
		});
	});
}
