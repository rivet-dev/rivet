import { describe, expect, test } from "vitest";
import type { DriverTestConfig } from "../mod";
import { setupDriverTest, waitFor } from "../utils";

const SLEEP_WAIT_MS = 150;
const REAL_TIMER_DB_TIMEOUT_MS = 180_000;

export function runActorDbPragmaMigrationTests(
	driverTestConfig: DriverTestConfig,
) {
	const dbTestTimeout = driverTestConfig.useRealTimers
		? REAL_TIMER_DB_TIMEOUT_MS
		: undefined;

	describe("Actor Database PRAGMA Migration Tests", () => {
		test(
			"applies all migrations on first start",
			async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);
				const actor = client.dbPragmaMigrationActor.getOrCreate([
					`pragma-init-${crypto.randomUUID()}`,
				]);

				// user_version should be set to 2 after migrations
				const version = await actor.getUserVersion();
				expect(version).toBe(2);

				// The status column from migration v2 should exist
				const columns = await actor.getColumns();
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
				const actor = client.dbPragmaMigrationActor.getOrCreate([
					`pragma-default-${crypto.randomUUID()}`,
				]);

				await actor.insertItem("test-item");
				const items = await actor.getItems();

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
				const actor = client.dbPragmaMigrationActor.getOrCreate([
					`pragma-explicit-${crypto.randomUUID()}`,
				]);

				await actor.insertItemWithStatus("done-item", "completed");
				const items = await actor.getItems();

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
				const actor = client.dbPragmaMigrationActor.getOrCreate([key]);

				// Insert data before sleep
				await actor.insertItemWithStatus("before-sleep", "pending");

				// Sleep and wake
				await actor.triggerSleep();
				await waitFor(driverTestConfig, SLEEP_WAIT_MS);

				// After wake, onMigrate runs again but should not fail
				const version = await actor.getUserVersion();
				expect(version).toBe(2);

				// Data should survive
				const items = await actor.getItems();
				expect(items).toHaveLength(1);
				expect(items[0].name).toBe("before-sleep");
				expect(items[0].status).toBe("pending");

				// Should still be able to insert
				await actor.insertItem("after-sleep");
				const items2 = await actor.getItems();
				expect(items2).toHaveLength(2);
			},
			dbTestTimeout,
		);
	});
}
