import { beforeEach, describe, expect, it } from "vitest";
import {
	InMemoryDriver,
	runWorkflow,
	type WorkflowContextInterface,
	type WorkflowHistorySnapshot,
} from "../src/testing.js";

const modes = ["yield", "live"] as const;

for (const mode of modes) {
	describe(`Workflow History Updates (${mode})`, { sequential: true }, () => {
		let driver: InMemoryDriver;

		beforeEach(() => {
			driver = new InMemoryDriver();
			driver.latency = 0;
		});

		it("emits history snapshots when entries change", async () => {
			const updates: WorkflowHistorySnapshot[] = [];

			const workflow = async (ctx: WorkflowContextInterface) => {
				await ctx.step("hello", async () => "world");
			};

			const result = await runWorkflow(
				"wf-history-1",
				workflow,
				undefined,
				driver,
				{
					mode,
					onHistoryUpdated: (snapshot) => {
						updates.push(snapshot);
					},
				},
			).result;

			expect(result.state).toBe("completed");
			expect(updates.length).toBeGreaterThan(0);

			const last = updates[updates.length - 1];
			expect(
				last.entries.some((entry) => entry.kind.type === "step"),
			).toBe(true);
			expect(
				Array.from(last.entryMetadata.values()).some(
					(meta) => meta.status === "completed",
				),
			).toBe(true);
		});
	});
}
