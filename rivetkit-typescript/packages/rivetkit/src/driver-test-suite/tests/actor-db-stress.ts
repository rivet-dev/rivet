import { describe, expect, test } from "vitest";
import { engineActorDriverNativeDatabaseAvailable } from "@/drivers/engine/actor-driver";
import type { DriverTestConfig } from "../mod";
import { setupDriverTest, waitFor } from "../utils";

const STRESS_TEST_TIMEOUT_MS = 60_000;

/**
 * Stress and resilience tests for the SQLite database subsystem.
 *
 * These tests target edge cases from the adversarial review:
 * - C1: close_database racing with in-flight operations
 * - H1: lifecycle operations blocking the Node.js event loop
 *
 * They run against the native runtime path.
 */
export function runActorDbStressTests(driverTestConfig: DriverTestConfig) {
	const nativeAvailable = engineActorDriverNativeDatabaseAvailable();
	describe("Actor Database Stress Tests", () => {
		test(
			"destroy during long-running DB operation completes without crash",
			async (c) => {
				const { client } = await setupDriverTest(
					c,
					driverTestConfig,
				);

				// Start multiple actors and kick off long DB operations,
				// then destroy them mid-flight. The test passes if no
				// actor crashes and no unhandled errors propagate.
				const actors = Array.from({ length: 5 }, (_, i) =>
					client.dbStressActor.getOrCreate([
						`stress-destroy-${i}-${crypto.randomUUID()}`,
					]),
				);

				// Start long-running inserts on all actors.
				const insertPromises = actors.map((actor) =>
					actor.insertBatch(500).catch((err: Error) => ({
						error: err.message,
					})),
				);

				// Immediately destroy all actors while inserts are in flight.
				const destroyPromises = actors.map((actor) =>
					actor.destroy().catch((err: Error) => ({
						error: err.message,
					})),
				);

				// Both sets of operations should resolve without hanging.
				// Inserts may succeed or fail with an error (actor destroyed),
				// but must not crash the process.
				const results = await Promise.allSettled([
					...insertPromises,
					...destroyPromises,
				]);

				// Verify all promises settled (none hung).
				expect(results).toHaveLength(10);
				for (const result of results) {
					expect(result.status).toBe("fulfilled");
				}
			},
			STRESS_TEST_TIMEOUT_MS,
		);

		test(
			"rapid create-insert-destroy cycles handle DB lifecycle correctly",
			async (c) => {
				const { client } = await setupDriverTest(
					c,
					driverTestConfig,
				);

				// Perform rapid cycles of create -> insert -> destroy.
				// This exercises the close_database path racing with
				// any pending DB operations from the insert.
				for (let i = 0; i < 10; i++) {
					const actor = client.dbStressActor.getOrCreate([
						`stress-cycle-${i}-${crypto.randomUUID()}`,
					]);

					// Insert some data.
					await actor.insertBatch(10);

					// Verify data was written.
					const count = await actor.getCount();
					expect(count).toBeGreaterThanOrEqual(10);

					// Destroy the actor (triggers close_database).
					await actor.destroy();
				}
			},
			STRESS_TEST_TIMEOUT_MS,
		);

		test(
			"DB operations complete without excessive blocking",
			async (c) => {
				const { client } = await setupDriverTest(
					c,
					driverTestConfig,
				);

				const actor = client.dbStressActor.getOrCreate([
					`stress-health-${crypto.randomUUID()}`,
				]);

				// Measure wall-clock time for 100 sequential DB inserts.
				// Each insert is an async round-trip through the VFS.
				// If lifecycle operations (open_database, close_database)
				// block the event loop, this will take much longer than
				// expected because the action itself runs on that loop.
				const health = await actor.measureEventLoopHealth(100);

				// 100 sequential inserts should complete in well under
				// 30 seconds. A blocked event loop (e.g., 30s WebSocket
				// timeout on open_database) would push this way over.
				expect(health.elapsedMs).toBeLessThan(30_000);
				expect(health.insertCount).toBe(100);

				// Verify the actor is still healthy after the test.
				const integrity = await actor.integrityCheck();
				expect(integrity.toLowerCase()).toBe("ok");
			},
			STRESS_TEST_TIMEOUT_MS,
		);

		// This test requires the engine driver's native database transport reset
		// hook. Dynamic isolates manage their own database transport separately.
		describe.skipIf(!nativeAvailable || driverTestConfig.isDynamic)(
			"Native Database Transport Resilience",
			() => {
				test(
					"recovers from forced native transport disconnect during DB writes",
					async (c) => {
						const { client, testEndpoint } =
							await setupDriverTest(c, driverTestConfig);

						const actor = client.dbStressActor.getOrCreate([
							`stress-disconnect-${crypto.randomUUID()}`,
						]);

						// Write initial data to confirm the actor works.
						await actor.insertBatch(10);
						expect(await actor.getCount()).toBe(10);

						// Force-close the native database transport handle.
						const res = await fetch(
							`${testEndpoint}/.test/native-db/force-disconnect`,
							{ method: "POST" },
						);
						expect(res.ok).toBe(true);
						const body = (await res.json()) as {
							closed: number;
						};
						expect(body.closed).toBeGreaterThanOrEqual(0);

						// Give the runtime a moment to reopen the transport.
						await waitFor(driverTestConfig, 2000);

						// The actor should still work after reconnection.
						await actor.insertBatch(10);
						const finalCount = await actor.getCount();
						expect(finalCount).toBe(20);

						// Verify data integrity after the disruption.
						const integrity = await actor.integrityCheck();
						expect(integrity.toLowerCase()).toBe("ok");
					},
					STRESS_TEST_TIMEOUT_MS,
				);

				test(
					"handles native transport disconnect during active write operation",
					async (c) => {
						const { client, testEndpoint } =
							await setupDriverTest(c, driverTestConfig);

						const actor = client.dbStressActor.getOrCreate([
							`stress-active-disconnect-${crypto.randomUUID()}`,
						]);

						// Confirm the actor is healthy.
						await actor.insertBatch(5);

						// Start a large write operation and disconnect
						// mid-flight. The write may fail, but the actor
						// should recover.
						const writePromise = actor
							.insertBatch(200)
							.catch((err: Error) => ({
								error: err.message,
							}));

						// Small delay to let the write start, then disconnect.
						await new Promise((resolve) =>
							setTimeout(resolve, 50),
						);

						await fetch(
							`${testEndpoint}/.test/native-db/force-disconnect`,
							{ method: "POST" },
						);

						// Wait for the write to settle (success or failure).
						await writePromise;

						// Wait for reconnection.
						await waitFor(driverTestConfig, 2000);

						// Actor should recover. New operations should work.
						await actor.insertBatch(5);
						const count = await actor.getCount();
						// At least the initial 5 + final 5 should exist.
						// The mid-disconnect 200 may or may not have committed.
						expect(count).toBeGreaterThanOrEqual(10);

						const integrity = await actor.integrityCheck();
						expect(integrity.toLowerCase()).toBe("ok");
					},
					STRESS_TEST_TIMEOUT_MS,
				);
			},
		);
	});
}
