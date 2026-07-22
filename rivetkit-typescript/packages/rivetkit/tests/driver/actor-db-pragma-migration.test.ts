import { describe, expect, test } from "vitest";
import { describeDriverMatrix } from "./shared-matrix";
import { setupDriverTest, waitFor } from "./shared-utils";

const SLEEP_WAIT_MS = 150;
const REAL_TIMER_DB_TIMEOUT_MS = 180_000;

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

				// The schema table should be set to 2 after migrations.
				const actor = getActor();
				await actor.ready;
				const version = await actor.getSchemaVersion();
				expect(version).toBe(2);

				// The status column from migration v2 should exist
				const columnsActor = getActor();
				await columnsActor.ready;
				const columns = await columnsActor.getColumns();
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

				const actor = getActor();
				await actor.ready;
				await actor.insertItem("test-item");

				const itemsActor = getActor();
				await itemsActor.ready;
				const items = await itemsActor.getItems();

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

				const actor = getActor();
				await actor.ready;
				await actor.insertItemWithStatus("done-item", "completed");

				const itemsActor = getActor();
				await itemsActor.ready;
				const items = await itemsActor.getItems();

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
				const actor = getActor();
				await actor.ready;
				await actor.insertItemWithStatus("before-sleep", "pending");

				// Sleep and wake
				await getActor().triggerSleep();
				await waitFor(driverTestConfig, SLEEP_WAIT_MS);

				// After wake, onMigrate runs again but should not fail
				const versionActor = getActor();
				await versionActor.ready;
				const version = await versionActor.getSchemaVersion();
				expect(version).toBe(2);

				// Data should survive
				const itemsActor = getActor();
				await itemsActor.ready;
				const items = await itemsActor.getItems();
				expect(items).toHaveLength(1);
				expect(items[0].name).toBe("before-sleep");
				expect(items[0].status).toBe("pending");

				// Should still be able to insert
				const insertActor = getActor();
				await insertActor.ready;
				await insertActor.insertItem("after-sleep");

				const items2Actor = getActor();
				await items2Actor.ready;
				const items2 = await items2Actor.getItems();
				expect(items2).toHaveLength(2);
			},
			dbTestTimeout,
		);
	});
});
