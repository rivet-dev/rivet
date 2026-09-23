import { beforeEach, describe, expect, it } from "vitest";
import {
	InMemoryDriver,
	type KVWrite,
	loadMetadata,
	loadStorage,
	runWorkflow,
	StepOutputTooLargeError,
	StorageLimitError,
	type WorkflowContextInterface,
	type WorkflowErrorEvent,
} from "../src/testing.js";

const modes = ["yield", "live"] as const;
const VALUE_LIMIT = 256 * 1024;

// Rejects whole batches like rivetkit-core's atomic state + workflow flush.
class AtomicLimitDriver extends InMemoryDriver {
	readonly atomicBatch = true;

	async batch(writes: KVWrite[]): Promise<void> {
		for (const { value } of writes) {
			if (value.byteLength > VALUE_LIMIT) {
				throw new StorageLimitError(
					`Workflow storage value too large (${value.byteLength} bytes). Limit is ${VALUE_LIMIT} bytes.`,
				);
			}
		}
		await super.batch(writes);
	}
}

const oversizedOutput = () => "x".repeat(VALUE_LIMIT + 1);

for (const mode of modes) {
	describe(
		`Workflow Engine Storage Limits (${mode})`,
		{ sequential: true },
		() => {
			let driver: AtomicLimitDriver;

			beforeEach(() => {
				driver = new AtomicLimitDriver();
				driver.latency = 0;
			});

			it("fails a step with an oversized output instead of wedging the workflow", async () => {
				let runs = 0;
				const events: WorkflowErrorEvent[] = [];
				const workflow = async (ctx: WorkflowContextInterface) =>
					await ctx.step("big-output", async () => {
						runs++;
						return oversizedOutput();
					});

				const error = await runWorkflow(
					"wf-1",
					workflow,
					undefined,
					driver,
					{
						mode,
						onError: (event) => {
							events.push(event);
						},
					},
				).result.catch((error: unknown) => error);

				expect(error).toBeInstanceOf(StepOutputTooLargeError);
				expect((error as Error).message).toMatch(
					/Step "big-output" output \(\d+ bytes serialized\) exceeds workflow storage limits: .*Limit is 262144 bytes/,
				);
				expect(runs).toBe(1);
				expect(events).toEqual([
					expect.objectContaining({
						step: expect.objectContaining({
							stepName: "big-output",
							attempt: 1,
							willRetry: false,
							error: expect.objectContaining({
								name: "StepOutputTooLargeError",
							}),
						}),
					}),
				]);

				const storage = await loadStorage(driver);
				expect(storage.state).toBe("failed");
				expect(storage.error?.name).toBe("StepOutputTooLargeError");
				const [entry] = [...storage.history.entries.values()];
				expect(entry.kind.type).toBe("step");
				if (entry.kind.type === "step") {
					expect(entry.kind.data.output).toBeUndefined();
					expect(entry.kind.data.error).toContain(
						"StepOutputTooLargeError",
					);
				}
				const metadata = await loadMetadata(storage, driver, entry.id);
				expect(metadata.status).toBe("exhausted");
				expect(metadata.completedAt).toBeUndefined();
			});

			it("lets tryStep catch an oversized output as a critical failure", async () => {
				const workflow = async (ctx: WorkflowContextInterface) => {
					const result = await ctx.tryStep("big-output", async () =>
						oversizedOutput(),
					);
					const after = await ctx.step("after", async () => "ok");
					return {
						ok: result.ok,
						failure: result.ok ? undefined : result.failure,
						after,
					};
				};

				const result = await runWorkflow(
					"wf-1",
					workflow,
					undefined,
					driver,
					{
						mode,
					},
				).result;

				expect(result.state).toBe("completed");
				expect(result.output).toMatchObject({
					ok: false,
					failure: {
						kind: "critical",
						stepName: "big-output",
						attempts: 1,
						error: { name: "StepOutputTooLargeError" },
					},
					after: "ok",
				});
			});

			it("fails oversized outputs on drivers that chunk batches", async () => {
				const chunkingDriver = new InMemoryDriver();
				chunkingDriver.latency = 0;
				const workflow = async (ctx: WorkflowContextInterface) =>
					await ctx.step("huge-output", async () =>
						"x".repeat(2 * 1024 * 1024),
					);

				await expect(
					runWorkflow("wf-1", workflow, undefined, chunkingDriver, {
						mode,
					}).result,
				).rejects.toThrow(StepOutputTooLargeError);
				expect((await loadStorage(chunkingDriver)).state).toBe(
					"failed",
				);
			});
		},
	);
}
