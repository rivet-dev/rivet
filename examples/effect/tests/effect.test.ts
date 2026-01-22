import { setupTest } from "rivetkit/test";
import { describe, expect, test } from "vitest";
import { registry } from "../src/actors/index.ts";

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

		const counter = client.counter.getOrCreate(["broadcast-counter"]);

		// Track broadcast events
		const events: number[] = [];
		counter.on("newCount", (count: number) => {
			events.push(count);
		});

		await counter.increment(10);
		await counter.increment(5);

		// Give time for events to propagate
		await new Promise((resolve) => setTimeout(resolve, 100));

		expect(events).toContain(10);
		expect(events).toContain(15);
	});
});

describe("user actor with Effect workflows", () => {
	test("create user with email and get email", async (ctx) => {
		const { client } = await setupTest(ctx, registry);

		const user = await client.user.create(["user-1"], {
			input: { email: "test@example.com" },
		});

		const email = await user.getEmail();
		expect(email).toBe("test@example.com");
	});

	test("update user email with workflow", async (ctx) => {
		const { client } = await setupTest(ctx, registry);

		const user = await client.user.create(["user-2"], {
			input: { email: "old@example.com" },
		});

		// Update email through workflow
		await user.updateEmail("new@example.com");

		const email = await user.getEmail();
		expect(email).toBe("new@example.com");
	});
});

describe("lifecycle-demo actor with Effect-wrapped hooks", () => {
	test("onCreate hook is called on actor creation", async (ctx) => {
		const { client } = await setupTest(ctx, registry);

		const demo = client.lifecycleDemo.getOrCreate(["lifecycle-1"]);

		// Give time for onCreate to complete
		await new Promise((resolve) => setTimeout(resolve, 100));

		const events = await demo.getEvents();
		expect(events).toContain("onCreate");
	});

	test("onConnect and onDisconnect hooks track connections", async (ctx) => {
		const { client } = await setupTest(ctx, registry);

		const demo = client.lifecycleDemo.getOrCreate(["lifecycle-2"]);

		// Give time for hooks to complete
		await new Promise((resolve) => setTimeout(resolve, 100));

		const count = await demo.getConnectionCount();
		// Connection count should be at least 1 (the current connection)
		expect(count).toBeGreaterThanOrEqual(1);
	});

	test("clearEvents action works", async (ctx) => {
		const { client } = await setupTest(ctx, registry);

		const demo = client.lifecycleDemo.getOrCreate(["lifecycle-3"]);

		// Get initial events (should have onCreate at minimum)
		await new Promise((resolve) => setTimeout(resolve, 100));
		const initialEvents = await demo.getEvents();
		expect(initialEvents.length).toBeGreaterThan(0);

		// Clear events
		await demo.clearEvents();

		const clearedEvents = await demo.getEvents();
		expect(clearedEvents).toEqual([]);
	});
});
