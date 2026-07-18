import { describe, expect, test } from "vitest";
import { ActorWorkflowDriver, chunkWorkflowWrites } from "./driver";

function write(keyBytes: number, valueBytes: number) {
	return {
		key: new Uint8Array(keyBytes),
		value: new Uint8Array(valueBytes),
	};
}

describe("workflow sqlite batching", () => {
	test.each([
		[127, 1],
		[128, 1],
		[129, 2],
	])("chunks %i rows into %i transaction(s)", (rowCount, chunkCount) => {
		const chunks = chunkWorkflowWrites(
			Array.from({ length: rowCount }, () => write(1, 1)),
		);
		expect(chunks).toHaveLength(chunkCount);
		expect(chunks.flat()).toHaveLength(rowCount);
		expect(chunks.every((chunk) => chunk.length <= 128)).toBe(true);
	});

	test("splits immediately above the 512 KiB payload budget", () => {
		// Each persisted key also includes the two-byte workflow namespace.
		const exact = [write(1, 256 * 1024 - 3), write(1, 256 * 1024 - 3)];
		expect(chunkWorkflowWrites(exact)).toHaveLength(1);

		const over = [write(1, 256 * 1024 - 3), write(1, 256 * 1024 - 2)];
		expect(chunkWorkflowWrites(over)).toHaveLength(2);
	});

	test("delegates actor state and the whole workflow batch to one atomic runtime call", async () => {
		let receivedWrites: Array<{ key: Uint8Array; value: Uint8Array }> = [];
		const sql = {
			query: async () => ({ columns: [], rows: [] }),
			execute: async () => [],
			run: async () => ({ changes: 0 }),
			executeBatch: async () => [],
		};
		const actor = {
			stateManager: {
				saveStateAndWorkflowBatch: async (
					writes: Array<{ key: Uint8Array; value: Uint8Array }>,
				) => {
					receivedWrites = writes;
				},
			},
			queueManager: {},
		};
		const runCtx = {
			sql,
			internalKeepAwake: async <T>(promise: Promise<T>) => await promise,
		};
		const driver = new ActorWorkflowDriver(actor as never, runCtx as never);
		const writes = [write(1, 1), write(2, 2)];
		await driver.batch(writes);
		expect(driver.atomicBatch).toBe(true);
		expect(receivedWrites).toBe(writes);
	});
});
