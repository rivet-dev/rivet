import { describe, expect, test, vi } from "vitest";
import type { ActorConnRaw } from "@/client/actor-conn";
import type { DriverTestConfig } from "../mod";
import { setupDriverTest } from "../utils";

export function runActorReconnectTests(driverTestConfig: DriverTestConfig) {
	describe("Actor Reconnection Tests", () => {
		test("should reconnect and preserve connection state after non-clean disconnect", async (c) => {
			const { client, endpoint } = await setupDriverTest(
				c,
				driverTestConfig,
			);

			// Create actor and connect
			const handle = client.counterConn.getOrCreate(["test-reconnect"]);
			const connection = handle.connect();

			// Set an initial count on the connection
			await connection.increment(5);

			// Verify connection count is 1
			const connCount1 = await connection.getConnectionCount();
			expect(connCount1).toBe(1);

			// Force disconnect (non-clean) - simulates network failure
			const connRaw = connection as unknown as ActorConnRaw;
			await forceUncleanDisconnect(
				endpoint,
				connRaw.actorId!,
				connRaw.connectionId!,
			);

			// Wait a bit for the disconnection to be processed
			await vi.waitFor(
				async () => {
					const countAfterReconnect = await connection.getCount();
					expect(countAfterReconnect).toBe(5); // Should preserve the count
				},
				{ timeout: 5000, interval: 100 },
			);

			// Verify connection count is still 1 (same connection reconnected)
			const connCount2 = await connection.getConnectionCount();
			expect(connCount2).toBe(1);

			// Verify we can still increment the counter
			const newCount = await connection.getCount();
			expect(newCount).toBe(5);

			// Clean up
			await connection.dispose();
		});

		test("should not preserve connection state after clean disconnect", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);

			// Create actor and connect
			const handle = client.counterConn.getOrCreate([
				"test-clean-disconnect",
			]);
			const connection = handle.connect();

			// Set an initial count on the connection
			await connection.increment(10);

			// Verify connection count is 1
			const connCount1 = await connection.getConnectionCount();
			expect(connCount1).toBe(1);

			// Clean disconnect
			await connection.dispose();

			// Wait a bit to ensure disconnection is processed
			await vi.waitFor(
				async () => {
					// Check that connection count is now 0
					const handle2 = client.counterConn.get([
						"test-clean-disconnect",
					]);
					const connCount = await handle2.getConnectionCount();
					// This counts the current action caller
					expect(connCount).toBe(1);
				},
				{ timeout: 5000 },
			);

			// Create a new connection
			const connection2 = handle.connect();

			// The count should be reset since it's a new connection
			const countNewConnection = await connection2.getCount();
			expect(countNewConnection).toBe(0); // Should be reset

			// Verify connection count is 1 again (new connection)
			const connCount3 = await connection2.getConnectionCount();
			expect(connCount3).toBe(1);

			// Clean up
			await connection2.dispose();
		});

		test("should handle multiple non-clean disconnects and reconnects", async (c) => {
			const { client, endpoint } = await setupDriverTest(
				c,
				driverTestConfig,
			);

			// Create actor and connect
			const handle = client.counterConn.getOrCreate([
				"test-multiple-reconnect",
			]);
			const connection = handle.connect();

			// Set an initial count
			await connection.setCount(100);

			// Perform multiple disconnect-reconnect cycles
			for (let i = 0; i < 3; i++) {
				// Increment before disconnect
				await connection.increment(1);

				// Force disconnect
				const connRaw = connection as unknown as ActorConnRaw;
				await forceUncleanDisconnect(
					endpoint,
					connRaw.actorId!,
					connRaw.connectionId!,
				);

				// Wait for reconnection and verify state is preserved
				await vi.waitFor(
					async () => {
						const countAfter = await connection.getCount();
						expect(countAfter).toBe(101 + i);
					},
					{ timeout: 5000 },
				);

				// Verify connection count remains 1
				const connCount = await connection.getConnectionCount();
				expect(connCount).toBe(1);
			}

			// Final verification
			const finalCount = await connection.getCount();
			expect(finalCount).toBe(103);

			// Clean up
			await connection.dispose();
		});
	});
}

async function forceUncleanDisconnect(
	endpoint: string,
	actorId: string,
	connId: string,
): Promise<void> {
	const response = await fetch(
		`${endpoint}/.test/force-disconnect?actor=${actorId}&conn=${connId}`,
		{
			method: "POST",
		},
	);

	if (!response.ok) {
		const text = await response.text();
		throw new Error(`Failed to force disconnect: ${text}`);
	}
}
