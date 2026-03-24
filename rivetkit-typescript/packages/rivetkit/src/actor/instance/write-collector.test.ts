import { describe, it, expect } from "vitest";
import { WriteCollector } from "./write-collector.js";

describe("WriteCollector", () => {
	function setup() {
		const calls: [string, [Uint8Array, Uint8Array][]][] = [];
		const fakeDriver = {
			kvBatchPut: async (
				actorId: string,
				entries: [Uint8Array, Uint8Array][],
			) => {
				calls.push([actorId, entries]);
			},
		} as any;
		const actorId = "test-actor-id";
		const collector = new WriteCollector(fakeDriver, actorId);
		return { calls, collector, actorId };
	}

	it("flush() with no entries does nothing", async () => {
		const { calls, collector } = setup();
		await collector.flush();
		expect(calls).toHaveLength(0);
	});

	it("flush() with entries calls kvBatchPut with all collected entries", async () => {
		const { calls, collector, actorId } = setup();

		const key1 = new Uint8Array([1, 2, 3]);
		const val1 = new Uint8Array([4, 5, 6]);
		const key2 = new Uint8Array([7, 8]);
		const val2 = new Uint8Array([9, 10]);

		collector.add(key1, val1);
		collector.add(key2, val2);
		await collector.flush();

		expect(calls).toHaveLength(1);
		expect(calls[0]![0]).toBe(actorId);
		expect(calls[0]![1]).toHaveLength(2);
		expect(calls[0]![1]![0]).toEqual([key1, val1]);
		expect(calls[0]![1]![1]).toEqual([key2, val2]);
	});

	it("multiple add() calls accumulate entries", async () => {
		const { calls, collector } = setup();

		collector.add(new Uint8Array([1]), new Uint8Array([2]));
		collector.add(new Uint8Array([3]), new Uint8Array([4]));
		collector.add(new Uint8Array([5]), new Uint8Array([6]));

		await collector.flush();

		expect(calls).toHaveLength(1);
		expect(calls[0]![1]).toHaveLength(3);
	});

	it("after flush(), entries are cleared and second flush is a no-op", async () => {
		const { calls, collector } = setup();

		collector.add(new Uint8Array([1]), new Uint8Array([2]));
		await collector.flush();
		expect(calls).toHaveLength(1);

		await collector.flush();
		expect(calls).toHaveLength(1);
	});
});
