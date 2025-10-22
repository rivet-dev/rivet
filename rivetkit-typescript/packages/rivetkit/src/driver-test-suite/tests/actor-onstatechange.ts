import { describe, expect, test } from "vitest";
import type { DriverTestConfig } from "@/driver-test-suite/mod";
import { setupDriverTest } from "@/driver-test-suite/utils";

export function runActorOnStateChangeTests(driverTestConfig: DriverTestConfig) {
	describe("Actor onStateChange Tests", () => {
		test("triggers onStateChange when state is modified", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);

			const actor = client.onStateChangeActor.getOrCreate();

			// Modify state - should trigger onChange
			await actor.setValue(10);

			// Check that onChange was called
			const changeCount = await actor.getChangeCount();
			expect(changeCount).toBe(1);
		});

		test("triggers onChange multiple times for multiple state changes", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);

			const actor = client.onStateChangeActor.getOrCreate();

			// Modify state multiple times
			await actor.incrementMultiple(3);

			// Check that onChange was called for each modification
			const changeCount = await actor.getChangeCount();
			expect(changeCount).toBe(3);
		});

		test("does NOT trigger onChange for read-only actions", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);

			const actor = client.onStateChangeActor.getOrCreate();

			// Set initial value
			await actor.setValue(5);

			// Read value without modifying - should NOT trigger onChange
			const value = await actor.getValue();
			expect(value).toBe(5);

			// Check that onChange was NOT called
			const changeCount = await actor.getChangeCount();
			expect(changeCount).toBe(1);
		});

		test("does NOT trigger onChange for computed values", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);

			const actor = client.onStateChangeActor.getOrCreate();

			// Set initial value
			await actor.setValue(3);

			// Check that onChange was called
			{
				const changeCount = await actor.getChangeCount();
				expect(changeCount).toBe(1);
			}

			// Compute value without modifying state - should NOT trigger onChange
			const doubled = await actor.getDoubled();
			expect(doubled).toBe(6);

			// Check that onChange was NOT called
			{
				const changeCount = await actor.getChangeCount();
				expect(changeCount).toBe(1);
			}
		});

		test("simple: connect, call action, dispose does NOT trigger onChange", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);

			const actor = client.onStateChangeActor.getOrCreate();

			// Connect to the actor
			const connection = await actor.connect();

			// Call an action that doesn't modify state
			const value = await connection.getValue();
			expect(value).toBe(0);

			// Dispose the connection
			await connection.dispose();

			// Verify that onChange was NOT triggered
			const changeCount = await actor.getChangeCount();
			expect(changeCount).toBe(0);
		});
	});
}
