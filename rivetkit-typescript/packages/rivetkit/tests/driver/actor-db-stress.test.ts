import { describeDriverMatrix } from "./shared-matrix";
import { describe, expect, test } from "vitest";
import { setupDriverTest } from "./shared-utils";

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
describeDriverMatrix("Actor Db Stress", (driverTestConfig) => {
	describe("Actor Database Stress Tests", () => {
		test(
			"destroy during long-running DB operation completes without crash",
			async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

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
				const { client } = await setupDriverTest(c, driverTestConfig);

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
				const { client } = await setupDriverTest(c, driverTestConfig);

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
	});
}, { encodings: ["bare"] });
