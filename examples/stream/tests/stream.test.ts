import { setupTest } from "rivetkit/test";
import { expect, test } from "vitest";
import { registry } from "../src/actors.ts";

test("Stream processor maintains top 3 values", async (ctx) => {
	const { client } = await setupTest(ctx, registry);
	const stream = client.streamProcessor.getOrCreate(["test-top3"]);

	// Initial state should be empty
	const initial = await stream.getTopValues();
	expect(initial).toEqual([]);

	// Add first value
	const result1 = await stream.addValue(10);
	expect(result1).toEqual([10]);

	// Add second value (lower)
	const result2 = await stream.addValue(5);
	expect(result2).toEqual([10, 5]);

	// Add third value (higher)
	const result3 = await stream.addValue(15);
	expect(result3).toEqual([15, 10, 5]);

	// Add fourth value (should replace lowest)
	const result4 = await stream.addValue(8);
	expect(result4).toEqual([15, 10, 8]);

	// Add fifth value (should replace middle)
	const result5 = await stream.addValue(12);
	expect(result5).toEqual([15, 12, 10]);
});

test("Stream processor tracks statistics correctly", async (ctx) => {
	const { client } = await setupTest(ctx, registry);
	const stream = client.streamProcessor.getOrCreate(["test-stats"]);

	// Initial stats
	const initialStats = await stream.getStats();
	expect(initialStats).toEqual({
		topValues: [],
		totalCount: 0,
		highestValue: null,
	});

	// Add some values
	await stream.addValue(20);
	await stream.addValue(30);
	await stream.addValue(10);

	const stats = await stream.getStats();
	expect(stats).toEqual({
		topValues: [30, 20, 10],
		totalCount: 3,
		highestValue: 30,
	});

	// Add more values to test count tracking
	await stream.addValue(5);
	await stream.addValue(25);

	const finalStats = await stream.getStats();
	expect(finalStats.totalCount).toBe(5);
	expect(finalStats.topValues).toEqual([30, 25, 20]);
	expect(finalStats.highestValue).toBe(30);
});

test("Stream processor handles duplicate values", async (ctx) => {
	const { client } = await setupTest(ctx, registry);
	const stream = client.streamProcessor.getOrCreate(["test-duplicates"]);

	// Add duplicate values
	await stream.addValue(10);
	await stream.addValue(10);
	await stream.addValue(10);

	const result = await stream.getTopValues();
	expect(result).toEqual([10, 10, 10]);

	const stats = await stream.getStats();
	expect(stats.totalCount).toBe(3);
	expect(stats.highestValue).toBe(10);
});

test("Stream processor reset functionality", async (ctx) => {
	const { client } = await setupTest(ctx, registry);
	const stream = client.streamProcessor.getOrCreate(["test-reset"]);

	// Add some values
	await stream.addValue(100);
	await stream.addValue(200);
	await stream.addValue(50);

	// Verify state before reset
	const beforeReset = await stream.getStats();
	expect(beforeReset.totalCount).toBe(3);
	expect(beforeReset.topValues).toEqual([200, 100, 50]);

	// Reset the stream
	const resetResult = await stream.reset();
	expect(resetResult).toEqual({
		topValues: [],
		totalCount: 0,
		highestValue: null,
	});

	// Verify state after reset
	const afterReset = await stream.getStats();
	expect(afterReset).toEqual({
		topValues: [],
		totalCount: 0,
		highestValue: null,
	});
});

test("Stream processor handles edge case values", async (ctx) => {
	const { client } = await setupTest(ctx, registry);
	const stream = client.streamProcessor.getOrCreate(["test-edge-cases"]);

	// Test with zero
	await stream.addValue(0);
	expect(await stream.getTopValues()).toEqual([0]);

	// Test with negative numbers
	await stream.addValue(-5);
	await stream.addValue(-1);
	expect(await stream.getTopValues()).toEqual([0, -1, -5]);

	// Test with very large numbers
	await stream.addValue(1000000);
	expect(await stream.getTopValues()).toEqual([1000000, 0, -1]);

	const stats = await stream.getStats();
	expect(stats.totalCount).toBe(4);
	expect(stats.highestValue).toBe(1000000);
});
