import { beforeEach, describe, expect, it } from "vitest";
import {
	InMemoryDriver,
	loadMetadata,
	loadStorage,
	runWorkflow,
	type WorkflowContextInterface,
} from "../src/testing.js";

const modes = ["yield", "live"] as const;

for (const mode of modes) {
	describe(`Workflow Engine Storage (${mode})`, { sequential: true }, () => {
		let driver: InMemoryDriver;

		beforeEach(() => {
			driver = new InMemoryDriver();
			driver.latency = 0;
		});

		it("should persist workflow output and state", async () => {
			const workflow = async (_ctx: WorkflowContextInterface) => {
				return "value";
			};

			await runWorkflow("wf-1", workflow, undefined, driver, { mode })
				.result;

			const storage = await loadStorage(driver);
			expect(storage.state).toBe("completed");
			expect(storage.output).toBe("value");
		});

		it("should persist workflow errors", async () => {
			const workflow = async (_ctx: WorkflowContextInterface) => {
				throw new Error("boom");
			};

			await expect(
				runWorkflow("wf-1", workflow, undefined, driver, { mode })
					.result,
			).rejects.toThrow("boom");

			const storage = await loadStorage(driver);
			expect(storage.state).toBe("failed");
			expect(storage.error?.message).toBe("boom");
		});

		it("should persist entry metadata and names", async () => {
			const workflow = async (ctx: WorkflowContextInterface) => {
				return await ctx.step("named-step", async () => "ok");
			};

			await runWorkflow("wf-1", workflow, undefined, driver, { mode })
				.result;

			const storage = await loadStorage(driver);
			expect(storage.nameRegistry).toContain("named-step");

			const entry = [...storage.history.entries.values()][0];
			const metadata = await loadMetadata(storage, driver, entry.id);
			expect(metadata.status).toBe("completed");
			expect(metadata.attempts).toBe(1);
		});
	});
}
