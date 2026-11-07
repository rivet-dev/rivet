import { describe, expect, it } from "vitest";
import { InMemoryDriver, runWorkflow } from "../src/testing.js";

function runStep(timeout: number) {
	const driver = new InMemoryDriver();
	driver.latency = 0;
	return runWorkflow(
		"wf-1",
		async (ctx) =>
			ctx.step({
				name: "work",
				timeout,
				run: async () => {
					await new Promise((resolve) => setTimeout(resolve, 50));
					return "done";
				},
			}),
		undefined,
		driver,
	).result;
}

describe("step timeout", () => {
	it("does not time out a step whose timeout is longer than setTimeout allows", async () => {
		const result = await runStep(30 * 24 * 60 * 60 * 1000);
		expect(result.state).toBe("completed");
		expect(result.output).toBe("done");
	});

	it("treats an Infinity timeout as no timeout", async () => {
		const result = await runStep(Number.POSITIVE_INFINITY);
		expect(result.state).toBe("completed");
		expect(result.output).toBe("done");
	});

	it("still times out a step that runs past a normal timeout", async () => {
		await expect(runStep(10)).rejects.toThrow(/timed out/i);
	});
});
