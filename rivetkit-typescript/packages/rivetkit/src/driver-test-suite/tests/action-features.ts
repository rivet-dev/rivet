import { describe, expect, test } from "vitest";
import type { ActorError } from "@/client/errors";
import type { DriverTestConfig } from "../mod";
import { setupDriverTest } from "../utils";

export function runActionFeaturesTests(driverTestConfig: DriverTestConfig) {
	describe("Action Features", () => {
		// TODO: These do not work with fake timers
		describe("Action Timeouts", () => {
			const usesFakeTimers = !driverTestConfig.useRealTimers;

			test("should timeout actions that exceed the configured timeout", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				// The quick action should complete successfully
				const quickResult = await client.shortTimeoutActor
					.getOrCreate()
					.quickAction();
				expect(quickResult).toBe("quick response");

				// The slow action should throw a timeout error
				await expect(
					client.shortTimeoutActor.getOrCreate().slowAction(),
				).rejects.toThrow("Action timed out");
			});

			test("should respect the default timeout", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				// This action should complete within the default timeout
				const result = await client.defaultTimeoutActor
					.getOrCreate()
					.normalAction();
				expect(result).toBe("normal response");
			});

			test("non-promise action results should not be affected by timeout", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				// Synchronous action should not be affected by timeout
				const result = await client.syncTimeoutActor
					.getOrCreate()
					.syncAction();
				expect(result).toBe("sync response");
			});

			test("should allow configuring different timeouts for different actors", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				// The short timeout actor should fail
				await expect(
					client.shortTimeoutActor.getOrCreate().slowAction(),
				).rejects.toThrow("Action timed out");

				// The longer timeout actor should succeed
				const result = await client.longTimeoutActor
					.getOrCreate()
					.delayedAction();
				expect(result).toBe("delayed response");
			});
		});

		describe("Action Sync & Async", () => {
			test("should support synchronous actions", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				const instance = client.syncActionActor.getOrCreate();

				// Test increment action
				let result = await instance.increment(5);
				expect(result).toBe(5);

				result = await instance.increment(3);
				expect(result).toBe(8);

				// Test getInfo action
				const info = await instance.getInfo();
				expect(info.currentValue).toBe(8);
				expect(typeof info.timestamp).toBe("number");

				// Test reset action (void return)
				await instance.reset();
				result = await instance.increment(0);
				expect(result).toBe(0);
			});

			test("should support asynchronous actions", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				const instance = client.asyncActionActor.getOrCreate();

				// Test delayed increment
				const result = await instance.delayedIncrement(5);
				expect(result).toBe(5);

				// Test fetch data
				const data = await instance.fetchData("test-123");
				expect(data.id).toBe("test-123");
				expect(typeof data.timestamp).toBe("number");

				// Test successful async operation
				const success = await instance.asyncWithError(false);
				expect(success).toBe("Success");

				// Test error in async operation
				try {
					await instance.asyncWithError(true);
					expect.fail("did not error");
				} catch (error) {
					expect((error as ActorError).message).toBe(
						"Intentional error",
					);
				}
			});

			test("should handle promises returned from actions correctly", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				const instance = client.promiseActor.getOrCreate();

				// Test resolved promise
				const resolvedValue = await instance.resolvedPromise();
				expect(resolvedValue).toBe("resolved value");

				// Test delayed promise
				const delayedValue = await instance.delayedPromise();
				expect(delayedValue).toBe("delayed value");

				// Test rejected promise
				await expect(instance.rejectedPromise()).rejects.toThrow(
					"promised rejection",
				);

				// Check state was updated by the delayed promise
				const results = await instance.getResults();
				expect(results).toContain("delayed");
			});
		});

		describe("Large Payloads", () => {
			test("should handle large request within size limit", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				const instance = client.largePayloadActor.getOrCreate();

				// Create a large payload that's under the default 64KB limit
				// Each item is roughly 60 bytes, so 800 items ≈ 48KB
				const items: string[] = [];
				for (let i = 0; i < 800; i++) {
					items.push(`Item ${i} with some additional text to increase size`);
				}

				const result = await instance.processLargeRequest({ items });

				expect(result.itemCount).toBe(800);
				expect(result.firstItem).toBe("Item 0 with some additional text to increase size");
				expect(result.lastItem).toBe("Item 799 with some additional text to increase size");
			});

			test("should reject request exceeding maxIncomingMessageSize", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				const instance = client.largePayloadActor.getOrCreate();

				// Create a payload that exceeds the default 64KB limit
				// Each item is roughly 60 bytes, so 1500 items ≈ 90KB
				const items: string[] = [];
				for (let i = 0; i < 1500; i++) {
					items.push(`Item ${i} with some additional text to increase size`);
				}

				await expect(
					instance.processLargeRequest({ items })
				).rejects.toThrow();
			});

			test("should handle large response", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				const instance = client.largePayloadActor.getOrCreate();

				// Request a large response (800 items ≈ 48KB)
				const result = await instance.getLargeResponse(800);

				expect(result.items).toHaveLength(800);
				expect(result.items[0]).toBe("Item 0 with some additional text to increase size");
				expect(result.items[799]).toBe("Item 799 with some additional text to increase size");
			});

			test("should reject response exceeding maxOutgoingMessageSize", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				const instance = client.largePayloadActor.getOrCreate();

				// Request a response that exceeds the default 64KB limit
				// Each item is roughly 60 bytes, so 1500 items ≈ 90KB
				await expect(
					instance.getLargeResponse(1500)
				).rejects.toThrow();
			});
		});
	});
}
