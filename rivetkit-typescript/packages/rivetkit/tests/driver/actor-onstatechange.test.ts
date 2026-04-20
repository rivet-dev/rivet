import { describeDriverMatrix } from "./shared-matrix";
import { describe, expect, test } from "vitest";
import { setupDriverTest } from "./shared-utils";

const ON_STATE_CHANGE_TEST_TIMEOUT_MS = 30_000;

describeDriverMatrix("Actor Onstatechange", (driverTestConfig) => {
	describe("Actor onStateChange Tests", () => {
		test("triggers onStateChange when state is modified", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);

			const actor = client.onStateChangeActor.getOrCreate();

			// Modify state - should trigger onChange
			await actor.setValue(10);

			// Check that onChange was called
			const changeCount = await actor.getChangeCount();
			expect(changeCount).toBe(1);
		}, ON_STATE_CHANGE_TEST_TIMEOUT_MS);

		test("triggers onChange multiple times for multiple state changes", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);

			const actor = client.onStateChangeActor.getOrCreate();

			// Modify state multiple times via separate actions. The lifecycle-hooks
			// suite already covers the same stable contract for repeated writes.
			await actor.setValue(1);
			await actor.setValue(2);
			await actor.setValue(3);

			// Check that onChange was called for each modification
			const changeCount = await actor.getChangeCount();
			expect(changeCount).toBe(3);
		}, ON_STATE_CHANGE_TEST_TIMEOUT_MS);

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
		}, ON_STATE_CHANGE_TEST_TIMEOUT_MS);

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
		}, ON_STATE_CHANGE_TEST_TIMEOUT_MS);

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
		}, ON_STATE_CHANGE_TEST_TIMEOUT_MS);
	});
});
