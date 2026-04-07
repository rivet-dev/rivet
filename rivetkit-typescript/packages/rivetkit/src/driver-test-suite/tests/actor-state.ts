import { describe, expect, test } from "vitest";
import type { DriverTestConfig } from "../mod";
import { setupDriverTest } from "../utils";

export function runActorStateTests(driverTestConfig: DriverTestConfig) {
	describe("Actor State Tests", () => {
		describe("State Persistence", () => {
			test("persists state between actor instances", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				// Create instance and increment
				const counterInstance = client.counter.getOrCreate();
				const initialCount = await counterInstance.increment(5);
				expect(initialCount).toBe(5);

				// Get a fresh reference to the same actor and verify state persisted
				const sameInstance = client.counter.getOrCreate();
				const persistedCount = await sameInstance.increment(3);
				expect(persistedCount).toBe(8);
			});

			test("restores state after actor disconnect/reconnect", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				// Create actor and set initial state
				const counterInstance = client.counter.getOrCreate();
				await counterInstance.increment(5);

				// Reconnect to the same actor
				const reconnectedInstance = client.counter.getOrCreate();
				const persistedCount = await reconnectedInstance.increment(0);
				expect(persistedCount).toBe(5);
			});

			test("maintains separate state for different actors", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				// Create first counter with specific key
				const counterA = client.counter.getOrCreate(["counter-a"]);
				await counterA.increment(5);

				// Create second counter with different key
				const counterB = client.counter.getOrCreate(["counter-b"]);
				await counterB.increment(10);

				// Verify state is separate
				const countA = await counterA.increment(0);
				const countB = await counterB.increment(0);
				expect(countA).toBe(5);
				expect(countB).toBe(10);
			});
		});
	});
}
