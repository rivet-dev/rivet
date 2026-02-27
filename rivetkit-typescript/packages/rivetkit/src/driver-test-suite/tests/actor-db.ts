import { describe, expect, test } from "vitest";
import type { DriverTestConfig } from "../mod";
import { setupDriverTest, waitFor } from "../utils";

type DbVariant = "raw" | "drizzle";

const CHUNK_SIZE = 4096;
const LARGE_PAYLOAD_SIZE = 32768;
const HIGH_VOLUME_COUNT = 1000;
const SLEEP_WAIT_MS = 150;
const LIFECYCLE_POLL_INTERVAL_MS = 25;
const LIFECYCLE_POLL_ATTEMPTS = 40;
const REAL_TIMER_DB_TIMEOUT_MS = 180_000;
const CHUNK_BOUNDARY_SIZES = [
	CHUNK_SIZE - 1,
	CHUNK_SIZE,
	CHUNK_SIZE + 1,
	2 * CHUNK_SIZE - 1,
	2 * CHUNK_SIZE,
	2 * CHUNK_SIZE + 1,
	4 * CHUNK_SIZE - 1,
	4 * CHUNK_SIZE,
	4 * CHUNK_SIZE + 1,
];
const SHRINK_GROW_INITIAL_ROWS = 16;
const SHRINK_GROW_REGROW_ROWS = 10;
const SHRINK_GROW_INITIAL_PAYLOAD = 4096;
const SHRINK_GROW_REGROW_PAYLOAD = 6144;
const HOT_ROW_COUNT = 10;
const HOT_ROW_UPDATES = 240;
const INTEGRITY_SEED_COUNT = 64;
const INTEGRITY_CHURN_COUNT = 120;

function getDbActor(
	client: Awaited<ReturnType<typeof setupDriverTest>>["client"],
	variant: DbVariant,
) {
	return variant === "raw" ? client.dbActorRaw : client.dbActorDrizzle;
}

export function runActorDbTests(driverTestConfig: DriverTestConfig) {
	const variants: DbVariant[] = ["raw", "drizzle"];
	const dbTestTimeout = driverTestConfig.useRealTimers
		? REAL_TIMER_DB_TIMEOUT_MS
		: undefined;
	const lifecycleTestTimeout = driverTestConfig.useRealTimers
		? REAL_TIMER_DB_TIMEOUT_MS
		: undefined;

	for (const variant of variants) {
		describe(`Actor Database (${variant}) Tests`, () => {
			test(
				"bootstraps schema on startup",
				async (c) => {
					const { client } = await setupDriverTest(c, driverTestConfig);
					const actor = getDbActor(client, variant).getOrCreate([
						`db-${variant}-bootstrap-${crypto.randomUUID()}`,
					]);

					const count = await actor.getCount();
					expect(count).toBe(0);
				},
				dbTestTimeout,
			);

			test(
				"supports CRUD, raw SQL, and multi-statement exec",
				async (c) => {
					const { client } = await setupDriverTest(c, driverTestConfig);
					const actor = getDbActor(client, variant).getOrCreate([
						`db-${variant}-crud-${crypto.randomUUID()}`,
					]);

					await actor.reset();

					const first = await actor.insertValue("alpha");
					const second = await actor.insertValue("beta");

					const values = await actor.getValues();
					expect(values.length).toBeGreaterThanOrEqual(2);
					expect(values.some((row) => row.value === "alpha")).toBeTruthy();
					expect(values.some((row) => row.value === "beta")).toBeTruthy();

					await actor.updateValue(first.id, "alpha-updated");
					const updated = await actor.getValue(first.id);
					expect(updated).toBe("alpha-updated");

					await actor.deleteValue(second.id);
					const count = await actor.getCount();
					if (driverTestConfig.useRealTimers) {
						expect(count).toBeGreaterThanOrEqual(1);
					} else {
						expect(count).toBe(1);
					}

					const rawCount = await actor.rawSelectCount();
					if (driverTestConfig.useRealTimers) {
						expect(rawCount).toBeGreaterThanOrEqual(1);
					} else {
						expect(rawCount).toBe(1);
					}

					const multiValue = await actor.multiStatementInsert("gamma");
					expect(multiValue).toBe("gamma-updated");
				},
				dbTestTimeout,
			);

			test(
				"handles transactions",
				async (c) => {
					const { client } = await setupDriverTest(c, driverTestConfig);
					const actor = getDbActor(client, variant).getOrCreate([
						`db-${variant}-tx-${crypto.randomUUID()}`,
					]);

					await actor.reset();
					await actor.transactionCommit("commit");
					expect(await actor.getCount()).toBe(1);

					await actor.transactionRollback("rollback");
					expect(await actor.getCount()).toBe(1);
				},
				dbTestTimeout,
			);

			test(
				"persists across sleep and wake cycles",
				async (c) => {
					const { client } = await setupDriverTest(c, driverTestConfig);
					const actor = getDbActor(client, variant).getOrCreate([
						`db-${variant}-sleep-${crypto.randomUUID()}`,
					]);

					await actor.reset();
					await actor.insertValue("sleepy");
					const baselineCount = await actor.getCount();
					expect(baselineCount).toBeGreaterThan(0);

					for (let i = 0; i < 3; i++) {
						await actor.triggerSleep();
						await waitFor(driverTestConfig, SLEEP_WAIT_MS);
						expect(await actor.getCount()).toBe(baselineCount);
					}
				},
				dbTestTimeout,
			);

			test(
				"completes onDisconnect DB writes before sleeping",
				async (c) => {
					const { client } = await setupDriverTest(c, driverTestConfig);
					const key = `db-${variant}-disconnect-${crypto.randomUUID()}`;

					const actor = getDbActor(client, variant).getOrCreate([key]);
					await actor.reset();
					await actor.configureDisconnectInsert(true, 250);

					await waitFor(driverTestConfig, SLEEP_WAIT_MS + 250);
					await actor.configureDisconnectInsert(false, 0);

					expect(await actor.getDisconnectInsertCount()).toBe(1);
				},
				dbTestTimeout,
			);

			test(
				"handles high-volume inserts",
				async (c) => {
					const { client } = await setupDriverTest(c, driverTestConfig);
					const actor = getDbActor(client, variant).getOrCreate([
						`db-${variant}-high-volume-${crypto.randomUUID()}`,
					]);

					await actor.reset();
					await actor.insertMany(HIGH_VOLUME_COUNT);
					const count = await actor.getCount();
					if (driverTestConfig.useRealTimers) {
						expect(count).toBeGreaterThanOrEqual(HIGH_VOLUME_COUNT);
					} else {
						expect(count).toBe(HIGH_VOLUME_COUNT);
					}
				},
				dbTestTimeout,
			);

			test(
				"handles payloads across chunk boundaries",
				async (c) => {
					const { client } = await setupDriverTest(c, driverTestConfig);
					const actor = getDbActor(client, variant).getOrCreate([
						`db-${variant}-chunk-${crypto.randomUUID()}`,
					]);

					await actor.reset();
					for (const size of CHUNK_BOUNDARY_SIZES) {
						const { id } = await actor.insertPayloadOfSize(size);
						const storedSize = await actor.getPayloadSize(id);
						expect(storedSize).toBe(size);
					}
				},
				dbTestTimeout,
			);

			test(
				"handles large payloads",
				async (c) => {
					const { client } = await setupDriverTest(c, driverTestConfig);
					const actor = getDbActor(client, variant).getOrCreate([
						`db-${variant}-large-${crypto.randomUUID()}`,
					]);

					await actor.reset();
					const { id } = await actor.insertPayloadOfSize(LARGE_PAYLOAD_SIZE);
					const storedSize = await actor.getPayloadSize(id);
					expect(storedSize).toBe(LARGE_PAYLOAD_SIZE);
				},
				dbTestTimeout,
			);

			test(
				"supports shrink and regrow workloads with vacuum",
				async (c) => {
					const { client } = await setupDriverTest(c, driverTestConfig);
					const actor = getDbActor(client, variant).getOrCreate([
						`db-${variant}-shrink-regrow-${crypto.randomUUID()}`,
					]);

					await actor.reset();
					await actor.vacuum();
					const baselinePages = await actor.getPageCount();

					await actor.insertPayloadRows(
						SHRINK_GROW_INITIAL_ROWS,
						SHRINK_GROW_INITIAL_PAYLOAD,
					);
					const grownPages = await actor.getPageCount();

					await actor.reset();
					await actor.vacuum();
					const shrunkPages = await actor.getPageCount();

					await actor.insertPayloadRows(
						SHRINK_GROW_REGROW_ROWS,
						SHRINK_GROW_REGROW_PAYLOAD,
					);
					const regrownPages = await actor.getPageCount();

					expect(grownPages).toBeGreaterThanOrEqual(baselinePages);
					expect(shrunkPages).toBeLessThanOrEqual(grownPages);
					expect(regrownPages).toBeGreaterThan(shrunkPages);
				},
				dbTestTimeout,
			);

			test(
				"handles repeated updates to the same row",
				async (c) => {
					const { client } = await setupDriverTest(c, driverTestConfig);
					const actor = getDbActor(client, variant).getOrCreate([
						`db-${variant}-updates-${crypto.randomUUID()}`,
					]);

					await actor.reset();
					const { id } = await actor.insertValue("base");
					const result = await actor.repeatUpdate(id, 50);
					expect(result.value).toBe("Updated 49");
					const value = await actor.getValue(id);
					expect(value).toBe("Updated 49");

					const hotRowIds: number[] = [];
					for (let i = 0; i < HOT_ROW_COUNT; i++) {
						const row = await actor.insertValue(`init-${i}`);
						hotRowIds.push(row.id);
					}

					const updatedRows = await actor.roundRobinUpdateValues(
						hotRowIds,
						HOT_ROW_UPDATES,
					);
					expect(updatedRows).toHaveLength(HOT_ROW_COUNT);
					for (const row of updatedRows) {
						expect(row.value).toMatch(/^v-\d+$/);
					}
				},
				dbTestTimeout,
			);

			test(
				"passes integrity checks after mixed workload and sleep",
				async (c) => {
					const { client } = await setupDriverTest(c, driverTestConfig);
					const actor = getDbActor(client, variant).getOrCreate([
						`db-${variant}-integrity-${crypto.randomUUID()}`,
					]);

					await actor.reset();
					await actor.runMixedWorkload(
						INTEGRITY_SEED_COUNT,
						INTEGRITY_CHURN_COUNT,
					);
					expect((await actor.integrityCheck()).toLowerCase()).toBe("ok");

					await actor.triggerSleep();
					await waitFor(driverTestConfig, SLEEP_WAIT_MS + 100);
					expect((await actor.integrityCheck()).toLowerCase()).toBe("ok");
				},
				dbTestTimeout,
			);
		});
	}

	describe("Actor Database Lifecycle Cleanup Tests", () => {
		test(
			"runs db provider cleanup on sleep",
			async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);
				const observer = client.dbLifecycleObserver.getOrCreate(["observer"]);

				const lifecycle = client.dbLifecycle.getOrCreate([
					`db-lifecycle-sleep-${crypto.randomUUID()}`,
				]);
				const actorId = await lifecycle.getActorId();

				const before = await observer.getCounts(actorId);

				await lifecycle.insertValue("before-sleep");
				await lifecycle.triggerSleep();
				await waitFor(driverTestConfig, SLEEP_WAIT_MS + 100);
				await lifecycle.ping();

				let after = before;
				for (let i = 0; i < LIFECYCLE_POLL_ATTEMPTS; i++) {
					after = await observer.getCounts(actorId);
					if (after.cleanup >= before.cleanup + 1) {
						break;
					}
					await waitFor(driverTestConfig, LIFECYCLE_POLL_INTERVAL_MS);
				}

				expect(after.create).toBeGreaterThanOrEqual(before.create);
				expect(after.migrate).toBeGreaterThanOrEqual(before.migrate);
				expect(after.cleanup).toBeGreaterThanOrEqual(before.cleanup + 1);
			},
			lifecycleTestTimeout,
		);

		test(
			"runs db provider cleanup on destroy",
			async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);
				const observer = client.dbLifecycleObserver.getOrCreate(["observer"]);

				const lifecycle = client.dbLifecycle.getOrCreate([
					`db-lifecycle-destroy-${crypto.randomUUID()}`,
				]);
				const actorId = await lifecycle.getActorId();
				const before = await observer.getCounts(actorId);

				await lifecycle.insertValue("before-destroy");
				await lifecycle.triggerDestroy();
				await waitFor(driverTestConfig, SLEEP_WAIT_MS + 100);

				let cleanupCount = before.cleanup;
				for (let i = 0; i < LIFECYCLE_POLL_ATTEMPTS; i++) {
					const counts = await observer.getCounts(actorId);
					cleanupCount = counts.cleanup;
					if (cleanupCount >= before.cleanup + 1) {
						break;
					}
					await waitFor(driverTestConfig, LIFECYCLE_POLL_INTERVAL_MS);
				}

				expect(cleanupCount).toBeGreaterThanOrEqual(before.cleanup + 1);
			},
			lifecycleTestTimeout,
		);

		test(
			"runs db provider cleanup when migration fails",
			async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);
				const observer = client.dbLifecycleObserver.getOrCreate(["observer"]);
				const beforeTotalCleanup = await observer.getTotalCleanupCount();
				const key = `db-lifecycle-migrate-failure-${crypto.randomUUID()}`;
				const lifecycle = client.dbLifecycleFailing.getOrCreate([key]);

				let threw = false;
				try {
					await lifecycle.ping();
				} catch {
					threw = true;
				}
				expect(threw).toBeTruthy();

				let cleanupCount = beforeTotalCleanup;
				for (let i = 0; i < LIFECYCLE_POLL_ATTEMPTS; i++) {
					cleanupCount = await observer.getTotalCleanupCount();
					if (cleanupCount >= beforeTotalCleanup + 1) {
						break;
					}
					await waitFor(driverTestConfig, LIFECYCLE_POLL_INTERVAL_MS);
				}

				expect(cleanupCount).toBeGreaterThanOrEqual(beforeTotalCleanup + 1);
			},
			lifecycleTestTimeout,
		);

		test(
			"handles parallel actor lifecycle churn",
			async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);
				const observer = client.dbLifecycleObserver.getOrCreate(["observer"]);

				const actorHandles = Array.from({ length: 12 }, (_, i) =>
					client.dbLifecycle.getOrCreate([
						`db-lifecycle-stress-${i}-${crypto.randomUUID()}`,
					]),
				);
				const actorIds = await Promise.all(
					actorHandles.map((handle) => handle.getActorId()),
				);

				await Promise.all(
					actorHandles.map((handle, i) => handle.insertValue(`phase-1-${i}`)),
				);
				await Promise.all(actorHandles.map((handle) => handle.triggerSleep()));
				await waitFor(driverTestConfig, SLEEP_WAIT_MS + 100);
				await Promise.all(
					actorHandles.map((handle, i) => handle.insertValue(`phase-2-${i}`)),
				);

				const survivors = actorHandles.slice(0, 6);
				const destroyed = actorHandles.slice(6);

				await Promise.all(destroyed.map((handle) => handle.triggerDestroy()));
				await Promise.all(survivors.map((handle) => handle.triggerSleep()));
				await waitFor(driverTestConfig, SLEEP_WAIT_MS + 100);
				await Promise.all(survivors.map((handle) => handle.ping()));

				const survivorCounts = await Promise.all(
					survivors.map((handle) => handle.getCount()),
				);
				for (const count of survivorCounts) {
					if (driverTestConfig.useRealTimers) {
						expect(count).toBeGreaterThanOrEqual(2);
					} else {
						expect(count).toBe(2);
					}
				}

				const lifecycleCleanup = new Map<string, number>();
				for (let i = 0; i < LIFECYCLE_POLL_ATTEMPTS; i++) {
					let allCleaned = true;
					for (const actorId of actorIds) {
						const counts = await observer.getCounts(actorId);
						lifecycleCleanup.set(actorId, counts.cleanup);
						if (counts.cleanup < 1) {
							allCleaned = false;
						}
					}

					if (allCleaned) {
						break;
					}
					await waitFor(driverTestConfig, LIFECYCLE_POLL_INTERVAL_MS);
				}

				for (const actorId of actorIds) {
					expect(lifecycleCleanup.get(actorId) ?? 0).toBeGreaterThanOrEqual(1);
				}
			},
			lifecycleTestTimeout,
		);
	});
}
