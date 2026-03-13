import { describe, expect, it } from "vitest";
import {
	Loop,
	loadStorage,
	replayWorkflowFromStep,
	runWorkflow,
	type WorkflowContextInterface,
} from "../src/index.js";
import { InMemoryDriver } from "../src/testing.js";

function findStepIdByName(
	storage: Awaited<ReturnType<typeof loadStorage>>,
	name: string,
): string | undefined {
	const nameIndex = storage.nameRegistry.indexOf(name);
	if (nameIndex === -1) {
		return undefined;
	}

	return Array.from(storage.history.entries.values()).find(
		(entry) =>
			entry.kind.type === "step" &&
			entry.location[entry.location.length - 1] === nameIndex,
	)?.id;
}

describe("replayWorkflowFromStep", () => {
	it("replays from the requested step", async () => {
		const driver = new InMemoryDriver();
		driver.latency = 0;

		const timeline: string[] = [];
		const workflow = async (ctx: WorkflowContextInterface) => {
			await ctx.step("one", async () => {
				timeline.push("one");
			});
			await ctx.step("two", async () => {
				timeline.push("two");
			});
			await ctx.step("three", async () => {
				timeline.push("three");
			});
		};

		await runWorkflow("wf-1", workflow, undefined, driver).result;
		timeline.length = 0;

		const storage = await loadStorage(driver);
		const stepTwoIndex = storage.nameRegistry.indexOf("two");
		const stepTwo = Array.from(storage.history.entries.values()).find(
			(entry) =>
				entry.kind.type === "step" &&
				entry.location[entry.location.length - 1] === stepTwoIndex,
		);
		expect(stepTwo).toBeDefined();

		const snapshot = await replayWorkflowFromStep(
			"wf-1",
			driver,
			stepTwo?.id,
		);

		expect(snapshot.entries.map((entry) => entry.id)).toHaveLength(1);

		await runWorkflow("wf-1", workflow, undefined, driver).result;
		expect(timeline).toEqual(["two", "three"]);
	});

	it("replays from the beginning when the target step is omitted", async () => {
		const driver = new InMemoryDriver();
		driver.latency = 0;

		const timeline: string[] = [];
		const workflow = async (ctx: WorkflowContextInterface) => {
			await ctx.step("one", async () => {
				timeline.push("one");
			});
			await ctx.step("two", async () => {
				timeline.push("two");
			});
		};

		await runWorkflow("wf-1", workflow, undefined, driver).result;
		timeline.length = 0;

		await replayWorkflowFromStep("wf-1", driver);
		await runWorkflow("wf-1", workflow, undefined, driver).result;

		expect(timeline).toEqual(["one", "two"]);
	});

	it("rewinds the enclosing loop when replaying a nested loop step", async () => {
		const driver = new InMemoryDriver();
		driver.latency = 0;

		const timeline: string[] = [];
		const workflow = async (ctx: WorkflowContextInterface) => {
			await ctx.step("before", async () => {
				timeline.push("before");
			});
			await ctx.loop({
				name: "repeat",
				state: 0,
				run: async (loopCtx, state) => {
					await loopCtx.step(`loop-step-${state}`, async () => {
						timeline.push(`loop-step-${state}`);
					});

					if (state >= 1) {
						return Loop.break(state);
					}

					return Loop.continue(state + 1);
				},
			});
			await ctx.step("after", async () => {
				timeline.push("after");
			});
		};

		await runWorkflow("wf-1", workflow, undefined, driver).result;
		timeline.length = 0;

		const storage = await loadStorage(driver);
		const targetStepId = findStepIdByName(storage, "loop-step-1");
		expect(targetStepId).toBeDefined();

		await replayWorkflowFromStep("wf-1", driver, targetStepId);
		await runWorkflow("wf-1", workflow, undefined, driver).result;

		expect(timeline).toEqual(["loop-step-0", "loop-step-1", "after"]);
	});

	it("allows replay from a step inside a sleeping loop", async () => {
		const driver = new InMemoryDriver();
		driver.latency = 0;
		driver.workerPollInterval = 0;

		const timeline: string[] = [];
		const workflow = async (ctx: WorkflowContextInterface) => {
			await ctx.step("before", async () => {
				timeline.push("before");
			});
			await ctx.loop({
				name: "repeat",
				state: 0,
				run: async (loopCtx, state) => {
					if (state === 0) {
						await loopCtx.step("loop-step-0", async () => {
							timeline.push("loop-step-0");
						});
						return Loop.continue(1);
					}

					await loopCtx.step("loop-step-1", async () => {
						timeline.push("loop-step-1");
					});
					await loopCtx.sleep("pause", 20);
					return Loop.break("done");
				},
			});
			await ctx.step("after", async () => {
				timeline.push("after");
			});
		};

		const firstRun = await runWorkflow(
			"wf-1",
			workflow,
			undefined,
			driver,
		).result;
		expect(firstRun.state).toBe("sleeping");
		timeline.length = 0;

		const storage = await loadStorage(driver);
		const targetStepId = findStepIdByName(storage, "loop-step-1");
		expect(targetStepId).toBeDefined();

		await expect(
			replayWorkflowFromStep("wf-1", driver, targetStepId),
		).resolves.toBeDefined();

		const replayRun = await runWorkflow(
			"wf-1",
			workflow,
			undefined,
			driver,
		).result;
		expect(replayRun.state).toBe("sleeping");

		await new Promise((resolve) => setTimeout(resolve, 30));

		const resumedRun = await runWorkflow(
			"wf-1",
			workflow,
			undefined,
			driver,
		).result;
		expect(resumedRun.state).toBe("completed");
		expect(resumedRun.output).toBeUndefined();
		expect(timeline).toEqual(["loop-step-0", "loop-step-1", "after"]);
	});
});
