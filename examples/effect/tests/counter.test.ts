import { setupTest } from "rivetkit/test";
import { describe, expect, test } from "vitest";
import { registry } from "../src/actors.ts";

describe("effect counter actor", () => {
	test("increment counter", async (ctx) => {
		const { client } = await setupTest(ctx, registry);

		const counter = client.counter.getOrCreate(["test-1"]);

		const count = await counter.increment(5);
		expect(count).toBe(5);

		const count2 = await counter.increment(3);
		expect(count2).toBe(8);
	});

	test("decrement counter", async (ctx) => {
		const { client } = await setupTest(ctx, registry);

		const counter = client.counter.getOrCreate(["test-2"]);

		await counter.increment(10);
		const count = await counter.decrement(3);
		expect(count).toBe(7);
	});

	test("reset counter", async (ctx) => {
		const { client } = await setupTest(ctx, registry);

		const counter = client.counter.getOrCreate(["test-3"]);

		await counter.increment(42);
		const count = await counter.reset();
		expect(count).toBe(0);

		// Resetting when already zero should still return 0
		const count2 = await counter.reset();
		expect(count2).toBe(0);
	});

	test("getCount returns current value", async (ctx) => {
		const { client } = await setupTest(ctx, registry);

		const counter = client.counter.getOrCreate(["test-4"]);

		const initial = await counter.getCount();
		expect(initial).toBe(0);

		await counter.increment(7);
		const after = await counter.getCount();
		expect(after).toBe(7);
	});

	test("batchIncrement applies sum of amounts", async (ctx) => {
		const { client } = await setupTest(ctx, registry);

		const counter = client.counter.getOrCreate(["test-5"]);

		const count = await counter.batchIncrement([1, 2, 3, 4, 5]);
		expect(count).toBe(15);

		const count2 = await counter.batchIncrement([10, 20]);
		expect(count2).toBe(45);
	});

	test("state persists across client instances for same key", async (ctx) => {
		const { client } = await setupTest(ctx, registry);

		const counter1 = client.counter.getOrCreate(["shared"]);
		await counter1.increment(100);

		const counter2 = client.counter.getOrCreate(["shared"]);
		const count = await counter2.getCount();
		expect(count).toBe(100);
	});

	test("different keys have independent state", async (ctx) => {
		const { client } = await setupTest(ctx, registry);

		const counterA = client.counter.getOrCreate(["a"]);
		const counterB = client.counter.getOrCreate(["b"]);

		await counterA.increment(10);
		await counterB.increment(20);

		const countA = await counterA.getCount();
		const countB = await counterB.getCount();

		expect(countA).toBe(10);
		expect(countB).toBe(20);
	});
});
