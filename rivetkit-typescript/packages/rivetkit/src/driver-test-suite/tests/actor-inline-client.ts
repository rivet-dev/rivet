import { describe, expect, test } from "vitest";
import type { DriverTestConfig } from "../mod";
import { setupDriverTest } from "../utils";

export function runActorInlineClientTests(driverTestConfig: DriverTestConfig) {
	describe("Actor Inline Client Tests", () => {
		describe("Stateless Client Calls", () => {
			test("should make stateless calls to other actors", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				// Create the inline client actor
				const inlineClientHandle = client.inlineClientActor.getOrCreate(
					["inline-client-test"],
				);

				// Test calling counter.increment via inline client
				const result = await inlineClientHandle.callCounterIncrement(5);
				expect(result).toBe(5);

				// Verify the counter state was actually updated
				const counterState = await inlineClientHandle.getCounterState();
				expect(counterState).toBe(5);

				// Check that messages were logged
				const messages = await inlineClientHandle.getMessages();
				expect(messages).toHaveLength(2);
				expect(messages[0]).toContain(
					"Called counter.increment(5), result: 5",
				);
				expect(messages[1]).toContain("Got counter state: 5");
			});

			test("should handle multiple stateless calls", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				// Create the inline client actor
				const inlineClientHandle = client.inlineClientActor.getOrCreate(
					["inline-client-multi"],
				);

				// Clear any existing messages
				await inlineClientHandle.clearMessages();

				// Make multiple calls
				const result1 =
					await inlineClientHandle.callCounterIncrement(3);
				const result2 =
					await inlineClientHandle.callCounterIncrement(7);
				const finalState = await inlineClientHandle.getCounterState();

				expect(result1).toBe(3);
				expect(result2).toBe(10); // 3 + 7
				expect(finalState).toBe(10);

				// Check messages
				const messages = await inlineClientHandle.getMessages();
				expect(messages).toHaveLength(3);
				expect(messages[0]).toContain(
					"Called counter.increment(3), result: 3",
				);
				expect(messages[1]).toContain(
					"Called counter.increment(7), result: 10",
				);
				expect(messages[2]).toContain("Got counter state: 10");
			});
		});

		describe("Stateful Client Calls", () => {
			test("should connect to other actors and receive events", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				// Create the inline client actor
				const inlineClientHandle = client.inlineClientActor.getOrCreate(
					["inline-client-stateful"],
				);

				// Clear any existing messages
				await inlineClientHandle.clearMessages();

				// Test stateful connection with events
				const result =
					await inlineClientHandle.connectToCounterAndIncrement(4);

				expect(result.result1).toBe(4);
				expect(result.result2).toBe(12); // 4 + 8
				expect(result.events).toEqual([4, 12]); // Should have received both events

				// Check that message was logged
				const messages = await inlineClientHandle.getMessages();
				expect(messages).toHaveLength(1);
				expect(messages[0]).toContain(
					"Connected to counter, incremented by 4 and 8",
				);
				expect(messages[0]).toContain("results: 4, 12");
				expect(messages[0]).toContain("events: [4,12]");
			});

			test("should handle stateful connection independently", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				// Create the inline client actor
				const inlineClientHandle = client.inlineClientActor.getOrCreate(
					["inline-client-independent"],
				);

				// Clear any existing messages
				await inlineClientHandle.clearMessages();

				// Test with different increment values
				const result =
					await inlineClientHandle.connectToCounterAndIncrement(2);

				expect(result.result1).toBe(2);
				expect(result.result2).toBe(6); // 2 + 4
				expect(result.events).toEqual([2, 6]);

				// Verify the state is independent from previous tests
				const messages = await inlineClientHandle.getMessages();
				expect(messages).toHaveLength(1);
				expect(messages[0]).toContain(
					"Connected to counter, incremented by 2 and 4",
				);
			});
		});

		describe("Mixed Client Usage", () => {
			test("should handle both stateless and stateful calls", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				// Create the inline client actor
				const inlineClientHandle = client.inlineClientActor.getOrCreate(
					["inline-client-mixed"],
				);

				// Clear any existing messages
				await inlineClientHandle.clearMessages();

				// Start with stateless calls
				await inlineClientHandle.callCounterIncrement(1);
				const statelessResult =
					await inlineClientHandle.getCounterState();
				expect(statelessResult).toBe(1);

				// Then do stateful call
				const statefulResult =
					await inlineClientHandle.connectToCounterAndIncrement(3);
				expect(statefulResult.result1).toBe(3);
				expect(statefulResult.result2).toBe(9); // 3 + 6

				// Check all messages were logged
				const messages = await inlineClientHandle.getMessages();
				expect(messages).toHaveLength(3);
				expect(messages[0]).toContain(
					"Called counter.increment(1), result: 1",
				);
				expect(messages[1]).toContain("Got counter state: 1");
				expect(messages[2]).toContain(
					"Connected to counter, incremented by 3 and 6",
				);
			});
		});
	});
}
