import { describe, expect, test } from "vitest";
import {
	INTERNAL_ERROR_CODE,
	INTERNAL_ERROR_DESCRIPTION,
} from "@/actor/errors";
import { assertUnreachable } from "@/actor/utils";
import type { DriverTestConfig } from "../mod";
import { setupDriverTest } from "../utils";

export function runActorErrorHandlingTests(driverTestConfig: DriverTestConfig) {
	describe("Actor Error Handling Tests", () => {
		describe("UserError Handling", () => {
			test("should handle simple UserError with message", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				// Try to call an action that throws a simple UserError
				const handle = client.errorHandlingActor.getOrCreate();

				try {
					await handle.throwSimpleError();
					// If we get here, the test should fail
					expect(true).toBe(false); // This should not be reached
				} catch (error: any) {
					// Verify the error properties
					expect(error.message).toBe("Simple error message");
					// Default code is "user_error" when not specified
					expect(error.code).toBe("user_error");
					// No metadata by default
					expect(error.metadata).toBeUndefined();
				}
			});

			test("should handle detailed UserError with code and metadata", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				// Try to call an action that throws a detailed UserError
				const handle = client.errorHandlingActor.getOrCreate();

				try {
					await handle.throwDetailedError();
					// If we get here, the test should fail
					expect(true).toBe(false); // This should not be reached
				} catch (error: any) {
					// Verify the error properties
					expect(error.message).toBe("Detailed error message");
					expect(error.code).toBe("detailed_error");
					expect(error.metadata).toBeDefined();
					expect(error.metadata.reason).toBe("test");
					expect(error.metadata.timestamp).toBeDefined();
				}
			});
		});

		describe("Internal Error Handling", () => {
			test("should convert internal errors to safe format", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				// Try to call an action that throws an internal error
				const handle = client.errorHandlingActor.getOrCreate();

				try {
					await handle.throwInternalError();
					// If we get here, the test should fail
					expect(true).toBe(false); // This should not be reached
				} catch (error: any) {
					// Verify the error is converted to a safe format
					expect(error.code).toBe(INTERNAL_ERROR_CODE);
					// Original error details should not be exposed
					expect(error.message).toBe(INTERNAL_ERROR_DESCRIPTION);
				}
			});
		});

		// TODO: Does not work with fake timers
		describe.skip("Action Timeout", () => {
			test("should handle action timeouts with custom duration", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				// Call an action that should time out
				const handle = client.errorHandlingActor.getOrCreate();

				// This should throw a timeout error because errorHandlingActor has
				// a 500ms timeout and this action tries to run for much longer
				const timeoutPromise = handle.timeoutAction();

				try {
					await timeoutPromise;
					// If we get here, the test failed - timeout didn't occur
					expect(true).toBe(false); // This should not be reached
				} catch (error: any) {
					// Verify it's a timeout error
					expect(error.message).toMatch(/timed out/i);
				}
			});

			test("should successfully run actions within timeout", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				// Call an action with a delay shorter than the timeout
				const handle = client.errorHandlingActor.getOrCreate();

				// This should succeed because 200ms < 500ms timeout
				const result = await handle.delayedAction(200);
				expect(result).toBe("Completed after 200ms");
			});

			test("should respect different timeouts for different actors", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				// The following actors have different timeout settings:
				// customTimeoutActor: 200ms timeout
				// standardTimeoutActor: default timeout (much longer)

				// This should fail - 300ms delay with 200ms timeout
				try {
					await client.customTimeoutActor.getOrCreate().slowAction();
					// Should not reach here
					expect(true).toBe(false);
				} catch (error: any) {
					expect(error.message).toMatch(/timed out/i);
				}

				// This should succeed - 50ms delay with 200ms timeout
				const quickResult = await client.customTimeoutActor
					.getOrCreate()
					.quickAction();
				expect(quickResult).toBe("Quick action completed");
			});
		});

		describe("Error Recovery", () => {
			test("should continue working after errors", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				const handle = client.errorHandlingActor.getOrCreate();

				// Trigger an error
				try {
					await handle.throwSimpleError();
				} catch (error) {
					// Ignore error
				}

				// Actor should still work after error
				const result = await handle.successfulAction();
				expect(result).toBe("success");
			});
		});
	});
}
