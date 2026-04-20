import { describeDriverMatrix } from "./shared-matrix";
import { describe, expect, test, vi } from "vitest";
import { setupDriverTest, waitFor } from "./shared-utils";

const SLEEP_WAIT_MS = 150;
const REAL_TIMER_DB_TIMEOUT_MS = 180_000;
const PRAGMA_READY_TIMEOUT_MS = 15_000;

async function waitForPragmaAction<T>(action: () => Promise<T>): Promise<T> {
	return await vi.waitFor(action, {
		timeout: PRAGMA_READY_TIMEOUT_MS,
		interval: 100,
	});
}

describeDriverMatrix("Actor Db Pragma Migration", (driverTestConfig) => {
	const dbTestTimeout = driverTestConfig.useRealTimers
		? REAL_TIMER_DB_TIMEOUT_MS
		: undefined;

	describe("Actor Database PRAGMA Migration Tests", () => {
		test(
			"applies all migrations on first start",
			async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);
				const key = `pragma-init-${crypto.randomUUID()}`;
				const getActor = () =>
					client.dbPragmaMigrationActor.getOrCreate([key]);

				// user_version should be set to 2 after migrations
				const version = await waitForPragmaAction(() =>
					getActor().getUserVersion(),
				);
				expect(version).toBe(2);

				// The status column from migration v2 should exist
				const columns = await waitForPragmaAction(() =>
					getActor().getColumns(),
				);
				expect(columns).toContain("id");
				expect(columns).toContain("name");
				expect(columns).toContain("status");
			},
			dbTestTimeout,
		);

		test(
			"inserts with default status from migration v2",
			async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);
				const key = `pragma-default-${crypto.randomUUID()}`;
				const getActor = () =>
					client.dbPragmaMigrationActor.getOrCreate([key]);

				await waitForPragmaAction(() =>
					getActor().insertItem("test-item"),
				);
				const items = await waitForPragmaAction(() =>
					getActor().getItems(),
				);

				expect(items).toHaveLength(1);
				expect(items[0].name).toBe("test-item");
				expect(items[0].status).toBe("active");
			},
			dbTestTimeout,
		);

		test(
			"inserts with explicit status",
			async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);
				const key = `pragma-explicit-${crypto.randomUUID()}`;
				const getActor = () =>
					client.dbPragmaMigrationActor.getOrCreate([key]);

				await waitForPragmaAction(() =>
					getActor().insertItemWithStatus("done-item", "completed"),
				);
				const items = await waitForPragmaAction(() =>
					getActor().getItems(),
				);

				expect(items).toHaveLength(1);
				expect(items[0].name).toBe("done-item");
				expect(items[0].status).toBe("completed");
			},
			dbTestTimeout,
		);

		test(
			"migrations are idempotent across sleep/wake",
			async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);
				const key = `pragma-sleep-${crypto.randomUUID()}`;
				const getActor = () =>
					client.dbPragmaMigrationActor.getOrCreate([key]);

				// Insert data before sleep
				await waitForPragmaAction(() =>
					getActor().insertItemWithStatus("before-sleep", "pending"),
				);

				// Sleep and wake
				await getActor().triggerSleep();
				await waitFor(driverTestConfig, SLEEP_WAIT_MS);

				// After wake, onMigrate runs again but should not fail
				const version = await waitForPragmaAction(() =>
					getActor().getUserVersion(),
				);
				expect(version).toBe(2);

				// Data should survive
				const items = await waitForPragmaAction(() =>
					getActor().getItems(),
				);
				expect(items).toHaveLength(1);
				expect(items[0].name).toBe("before-sleep");
				expect(items[0].status).toBe("pending");

				// Should still be able to insert
				await waitForPragmaAction(() =>
					getActor().insertItem("after-sleep"),
				);
				const items2 = await waitForPragmaAction(() =>
					getActor().getItems(),
				);
				expect(items2).toHaveLength(2);
			},
			dbTestTimeout,
		);
	});
});
