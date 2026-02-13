import { describe, expect, test } from "vitest";
import type { DriverTestConfig } from "../mod";
import { setupDriverTest } from "../utils";

export function runConnErrorSerializationTests(driverTestConfig: DriverTestConfig) {
	describe("Connection Error Serialization Tests", () => {
		test("error thrown in createConnState preserves group and code through WebSocket serialization", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);

			const actorKey = `test-error-serialization-${Date.now()}`;

			// Create actor handle with params that will trigger error in createConnState
			const actor = client.connErrorSerializationActor.getOrCreate(
				[actorKey],
				{ params: { shouldThrow: true } },
			);

			// Try to connect, which will trigger error in createConnState
			const conn = actor.connect();

			// Wait for connection to fail
			let caughtError: any;
			try {
				// Try to call an action, which should fail because connection couldn't be established
				await conn.getValue();
			} catch (err) {
				caughtError = err;
			}

			// Verify the error was caught
			expect(caughtError).toBeDefined();

			// Verify the error has the correct group and code from the original error
			// Original error: new CustomConnectionError("...") with group="connection", code="custom_error"
			expect(caughtError.group).toBe("connection");
			expect(caughtError.code).toBe("custom_error");

			// Clean up
			await conn.dispose();
		});

		test("successful createConnState does not throw error", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);

			const actorKey = `test-no-error-${Date.now()}`;

			// Create actor handle with params that will NOT trigger error
			const actor = client.connErrorSerializationActor.getOrCreate(
				[actorKey],
				{ params: { shouldThrow: false } },
			);

			// Connect without triggering error
			const conn = actor.connect();

			// This should succeed
			const value = await conn.getValue();
			expect(value).toBe(0);

			// Clean up
			await conn.dispose();
		});
	});
}
