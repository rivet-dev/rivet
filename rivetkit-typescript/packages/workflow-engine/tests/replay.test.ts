import { describe, expect, it } from "vitest";
import {
	loadStorage,
	replayWorkflowFromStep,
	runWorkflow,
	type WorkflowContextInterface,
} from "../src/index.js";
import { InMemoryDriver } from "../src/testing.js";

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
});
