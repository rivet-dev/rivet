import { beforeEach, describe, expect, it } from "vitest";
import type { KVWrite } from "../src/driver.js";
import {
	MAX_KV_BATCH_ENTRIES,
	MAX_KV_BATCH_PAYLOAD_BYTES,
} from "../src/storage.js";
import {
	appendName,
	createEntry,
	createStorage,
	emptyLocation,
	flush,
	getOrCreateMetadata,
	InMemoryDriver,
	loadMetadata,
	loadStorage,
	runWorkflow,
	setEntry,
	type WorkflowContextInterface,
} from "../src/testing.js";

class RecordingDriver extends InMemoryDriver {
	batches: KVWrite[][] = [];
	failOnBatch?: number;

	override async batch(writes: KVWrite[]): Promise<void> {
		this.batches.push(writes);
		if (
			this.failOnBatch !== undefined &&
			this.batches.length === this.failOnBatch
		) {
			throw new Error("injected batch failure");
		}
		await super.batch(writes);
	}
}

function batchPayloadBytes(writes: KVWrite[]): number {
	return writes.reduce(
		(total, write) => total + write.key.byteLength + write.value.byteLength,
		0,
	);
}

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

describe("Workflow Engine Storage flush", () => {
	it("splits writes into KV-sized batches", async () => {
		const driver = new RecordingDriver();
		driver.latency = 0;
		const storage = createStorage();
		storage.flushedState = storage.state;
		storage.nameRegistry = Array.from(
			{ length: MAX_KV_BATCH_ENTRIES + 1 },
			(_, i) => `step-${i}`,
		);

		await flush(storage, driver);

		expect(driver.batches).toHaveLength(2);
		expect(driver.batches.map((batch) => batch.length)).toEqual([
			MAX_KV_BATCH_ENTRIES,
			1,
		]);
		for (const batch of driver.batches) {
			expect(batchPayloadBytes(batch)).toBeLessThanOrEqual(
				MAX_KV_BATCH_PAYLOAD_BYTES,
			);
		}

		const reloaded = await loadStorage(driver);
		expect(reloaded.nameRegistry).toEqual(storage.nameRegistry);
	});

	it("splits writes by KV batch payload size", async () => {
		const driver = new RecordingDriver();
		driver.latency = 0;
		const storage = createStorage();
		storage.flushedState = storage.state;
		storage.nameRegistry = Array.from(
			{ length: 9 },
			(_, i) => `${i}-${"x".repeat(120 * 1024)}`,
		);

		await flush(storage, driver);

		expect(driver.batches.length).toBeGreaterThan(1);
		for (const batch of driver.batches) {
			expect(batch.length).toBeLessThanOrEqual(MAX_KV_BATCH_ENTRIES);
			expect(batchPayloadBytes(batch)).toBeLessThanOrEqual(
				MAX_KV_BATCH_PAYLOAD_BYTES,
			);
		}

		const reloaded = await loadStorage(driver);
		expect(reloaded.nameRegistry).toEqual(storage.nameRegistry);
	});

	it("keeps dirty markers when a batch write fails", async () => {
		const driver = new RecordingDriver();
		driver.latency = 0;
		driver.failOnBatch = 1;
		const storage = createStorage();
		const location = appendName(storage, emptyLocation(), "step");
		const entry = createEntry(location, {
			type: "step",
			data: { output: "ok" },
		});
		setEntry(storage, location, entry);
		const metadata = getOrCreateMetadata(storage, entry.id);
		metadata.status = "completed";

		await expect(flush(storage, driver)).rejects.toThrow(
			"injected batch failure",
		);

		expect(entry.dirty).toBe(true);
		expect(metadata.dirty).toBe(true);
		expect(storage.flushedNameCount).toBe(0);

		driver.failOnBatch = undefined;
		await flush(storage, driver);

		expect(entry.dirty).toBe(false);
		expect(metadata.dirty).toBe(false);
	});
});
