import { describe, expect, test } from "vitest";
import { setupTest } from "rivetkit/test";
import { registry } from "../fixtures/registry.ts";

describe("Effect-wrapped actors", () => {
	describe("simple actor without Effect", () => {
		test("getValue and setValue work", async (ctx) => {
			const { client } = await setupTest(ctx, registry);

			const simple = client.simple.getOrCreate(["test-simple"]);

			// Initial value should be 0
			const initialValue = await simple.getValue();
			expect(initialValue).toBe(0);

			// Set to 42
			const newValue = await simple.setValue(42);
			expect(newValue).toBe(42);

			// Get should return 42
			const finalValue = await simple.getValue();
			expect(finalValue).toBe(42);
		});
	});

	describe("counter actor with Effect-wrapped actions", () => {
		test("increment counter and get count", async (ctx) => {
			const { client } = await setupTest(ctx, registry);

			const counter = client.counter.getOrCreate(["test-counter"]);

			// Initial count should be 0
			const initialCount = await counter.getCount();
			expect(initialCount).toBe(0);

			// Increment by 5
			const newCount = await counter.increment(5);
			expect(newCount).toBe(5);

			// Increment again
			const finalCount = await counter.increment(3);
			expect(finalCount).toBe(8);
		});

		test("counter broadcasts events on increment", async (ctx) => {
			const { client } = await setupTest(ctx, registry);

			// Use a unique key to avoid state pollution
			const uniqueKey = `broadcast-counter-${crypto.randomUUID()}`;
			const counter = client.counter.getOrCreate([uniqueKey]);

			// Connect to receive events
			const conn = counter.connect();

			// Track broadcast events
			const events: number[] = [];
			conn.on("newCount", (count: number) => {
				events.push(count);
			});

			// Wait for connection to be established
			await new Promise((resolve) => setTimeout(resolve, 50));

			await counter.increment(10);
			await counter.increment(5);

			// Give time for events to propagate
			await new Promise((resolve) => setTimeout(resolve, 100));

			expect(events).toContain(10);
			expect(events).toContain(15);
		});
	});
});
