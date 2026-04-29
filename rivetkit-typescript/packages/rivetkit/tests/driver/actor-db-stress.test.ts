import { describeDriverMatrix } from "./shared-matrix";
import { describe, expect, test, vi } from "vitest";
import { setupDriverTest, waitFor } from "./shared-utils";

const STRESS_TEST_TIMEOUT_MS = 60_000;
const KITCHEN_SINK_TEST_TIMEOUT_MS = 120_000;
const ACTOR_READY_TIMEOUT_MS = 15_000;
const RUNTIME_LOG_TAIL_CHARS = 20_000;

async function withRuntimeLogTail<T>(
	getRuntimeOutput: () => string,
	fn: () => Promise<T>,
): Promise<T> {
	try {
		return await fn();
	} catch (error) {
		const runtimeOutput = getRuntimeOutput();
		const runtimeTail = runtimeOutput.slice(-RUNTIME_LOG_TAIL_CHARS);
		if (error instanceof Error && runtimeTail) {
			error.message = `${error.message}\n\nRuntime log tail:\n${runtimeTail}`;
		}
		throw error;
	}
}

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
				const { client, getRuntimeOutput } = await setupDriverTest(
					c,
					driverTestConfig,
				);

				// Perform rapid cycles of create -> insert -> destroy.
				// This exercises the close_database path racing with
				// any pending DB operations from the insert.
				for (let i = 0; i < 10; i++) {
					const actorKey = [
						`stress-cycle-${i}-${crypto.randomUUID()}`,
					];
					const getActor = () => client.dbStressActor.getOrCreate(actorKey);

					// Poll the first insert because the actor can still be starting when the initial DB action is sent.
					await vi.waitFor(
						async () => {
							await withRuntimeLogTail(
								getRuntimeOutput,
								() => getActor().insertBatch(10),
							);
						},
						{ timeout: ACTOR_READY_TIMEOUT_MS, interval: 100 },
					);

					// Reacquire the keyed handle before verifying the write.
					// The direct target from the insert can already be moving
					// through sleep teardown under the task model.
					await vi.waitFor(
						async () => {
							const count = await withRuntimeLogTail(
								getRuntimeOutput,
								() =>
									client.dbStressActor
										.getOrCreate(actorKey)
										.getCount(),
							);
							expect(count).toBeGreaterThanOrEqual(10);
						},
						{ timeout: ACTOR_READY_TIMEOUT_MS, interval: 100 },
					);

					// Destroy the actor (triggers close_database).
					await getActor().destroy();
				}
			},
			STRESS_TEST_TIMEOUT_MS,
		);

		test(
			"DB operations complete without excessive blocking",
			async (c) => {
				const { client, getRuntimeOutput } = await setupDriverTest(
					c,
					driverTestConfig,
				);

				const actorKey = [`stress-health-${crypto.randomUUID()}`];

				// Measure wall-clock time for 100 sequential DB inserts.
				// Each insert is an async round-trip through the VFS.
				// If lifecycle operations (open_database, close_database)
				// block the event loop, this will take much longer than
				// expected because the action itself runs on that loop.
				const health = await vi.waitFor(
					async () =>
						withRuntimeLogTail(getRuntimeOutput, () =>
							client.dbStressActor
								.getOrCreate(actorKey)
								.measureEventLoopHealth(100),
						),
					{ timeout: ACTOR_READY_TIMEOUT_MS, interval: 100 },
				);

				// 100 sequential inserts should complete in well under
				// 30 seconds. A blocked event loop (e.g., 30s WebSocket
				// timeout on open_database) would push this way over.
				expect(health.elapsedMs).toBeLessThan(30_000);
				expect(health.insertCount).toBe(100);

				// Poll the integrity check because the actor may still be finishing the prior async insert loop.
				const integrity = await vi.waitFor(
					async () =>
						withRuntimeLogTail(getRuntimeOutput, () =>
							client.dbStressActor
								.getOrCreate(actorKey)
								.integrityCheck(),
						),
					{ timeout: ACTOR_READY_TIMEOUT_MS, interval: 100 },
				);
				expect(integrity.toLowerCase()).toBe("ok");
			},
			STRESS_TEST_TIMEOUT_MS,
		);

		test(
			"repeated autocommit upserts keep sqlite head txid consistent",
			async (c) => {
				const { client, getRuntimeOutput } = await setupDriverTest(
					c,
					driverTestConfig,
				);
				const actor = client.dbStressActor.getOrCreate([
					`stress-autocommit-upsert-${crypto.randomUUID()}`,
				]);

				await actor.reset();

				const count = await withRuntimeLogTail(
					getRuntimeOutput,
					() => actor.upsertMetaRows(240),
				);
				expect(count).toBe(32);

				const integrity = await withRuntimeLogTail(
					getRuntimeOutput,
					() => actor.integrityCheck(),
				);
				expect(integrity.toLowerCase()).toBe("ok");
			},
			STRESS_TEST_TIMEOUT_MS,
		);

		test(
			"kitchen sink sqlite smoke survives write churn and wake",
			async (c) => {
				const { client, getRuntimeOutput } = await setupDriverTest(
					c,
					driverTestConfig,
				);
				const actor = client.dbStressActor.getOrCreate([
					`stress-kitchen-sink-${crypto.randomUUID()}`,
				]);

				await actor.reset();

				const first = await withRuntimeLogTail(
					getRuntimeOutput,
					() => actor.kitchenSinkSmoke(320),
				);
				expect(first.metaCount).toBeGreaterThanOrEqual(19);
				expect(first.dataCount).toBeGreaterThan(0);
				expect(first.payloadCount).toBeGreaterThan(0);
				expect(first.pageCount).toBeGreaterThan(0);
				expect(first.integrity.toLowerCase()).toBe("ok");

				const burst = await withRuntimeLogTail(getRuntimeOutput, () =>
					Promise.all([
						actor.upsertMetaRows(320),
						actor.kitchenSinkSmoke(96),
						actor.upsertMetaRows(320),
					]),
				);
				expect(burst[0]).toBeGreaterThanOrEqual(32);
				expect(burst[1].integrity.toLowerCase()).toBe("ok");
				expect(burst[2]).toBeGreaterThanOrEqual(32);

				await actor.triggerSleep();
				await waitFor(driverTestConfig, 250);

				// Poll because the actor can still be in the stopping window after triggerSleep.
				const afterWake = await vi.waitFor(
					async () =>
						await withRuntimeLogTail(
							getRuntimeOutput,
							() => actor.kitchenSinkSmoke(96),
						),
					{ timeout: ACTOR_READY_TIMEOUT_MS, interval: 100 },
				);
				expect(afterWake.metaCount).toBeGreaterThanOrEqual(
					first.metaCount,
				);
				expect(afterWake.dataCount).toBeGreaterThan(first.dataCount);
				expect(afterWake.payloadCount).toBeGreaterThan(0);
				expect(afterWake.pageCount).toBeGreaterThan(0);
				expect(afterWake.integrity.toLowerCase()).toBe("ok");
			},
			KITCHEN_SINK_TEST_TIMEOUT_MS,
		);
	});
}, { encodings: ["bare"] });
