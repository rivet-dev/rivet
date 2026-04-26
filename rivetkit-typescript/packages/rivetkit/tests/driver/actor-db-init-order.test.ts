import { describe, expect, test } from "vitest";
import { describeDriverMatrix } from "./shared-matrix";
import { setupDriverTest, waitFor } from "./shared-utils";

const REAL_TIMER_DB_TIMEOUT_MS = 180_000;
const SLEEP_WAIT_MS = 150;
const LIFECYCLE_POLL_INTERVAL_MS = 25;
const LIFECYCLE_POLL_ATTEMPTS = 40;

describeDriverMatrix("Actor Db Init Order", (driverTestConfig) => {
	const dbTestTimeout = driverTestConfig.useRealTimers
		? REAL_TIMER_DB_TIMEOUT_MS
		: undefined;

	describe("onMigrate runs before lifecycle hooks", () => {
		test(
			"createState can read schema created by onMigrate",
			async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);
				const key = `db-init-order-create-state-${crypto.randomUUID()}`;
				const actor = client.dbInitOrderCreateStateActor.getOrCreate([
					key,
				]);

				const initialCount = await actor.getInitialCount();
				expect(initialCount).toBe(0);
			},
			dbTestTimeout,
		);

		test(
			"onCreate can read schema created by onMigrate",
			async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);
				const key = `db-init-order-on-create-${crypto.randomUUID()}`;
				const actor = client.dbInitOrderOnCreateActor.getOrCreate([key]);

				const initialCount = await actor.getInitialCount();
				expect(initialCount).toBe(0);
			},
			dbTestTimeout,
		);

		test(
			"createVars can read schema created by onMigrate",
			async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);
				const key = `db-init-order-create-vars-${crypto.randomUUID()}`;
				const actor = client.dbInitOrderCreateVarsActor.getOrCreate([
					key,
				]);

				const initialCount = await actor.getInitialCount();
				expect(initialCount).toBe(0);
			},
			dbTestTimeout,
		);
	});

	describe("c.db is usable from teardown and wake hooks", () => {
		test(
			"onWake can access c.db on initial create and on wake",
			async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);
				const key = `db-init-order-on-wake-${crypto.randomUUID()}`;
				const actor = client.dbInitOrderOnWakeActor.getOrCreate([key]);

				// onWake fires once on initial create.
				expect(await actor.getWakeCount()).toBe(1);

				// Sleep, then wake by issuing another action; onWake fires
				// again on wake.
				await actor.triggerSleep();
				await waitFor(driverTestConfig, SLEEP_WAIT_MS);

				let wakeCount = -1;
				for (let i = 0; i < LIFECYCLE_POLL_ATTEMPTS; i++) {
					wakeCount = await actor.getWakeCount();
					if (wakeCount >= 2) break;
					await waitFor(driverTestConfig, LIFECYCLE_POLL_INTERVAL_MS);
				}
				expect(wakeCount).toBe(2);
			},
			dbTestTimeout,
		);

		test(
			"onSleep can access c.db",
			async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);
				const observer = client.dbInitOrderObserver.getOrCreate([
					"db-init-order-observer",
				]);
				const key = `db-init-order-on-sleep-${crypto.randomUUID()}`;
				const actor = client.dbInitOrderOnSleepActor.getOrCreate([key]);
				const actorId = await actor.getActorId();

				await actor.insert("data1");
				await actor.insert("data2");

				await actor.triggerSleep();
				await waitFor(driverTestConfig, SLEEP_WAIT_MS);

				// onSleep records the row count it observed; verify it saw the
				// rows we inserted before sleep.
				let observed = -1;
				for (let i = 0; i < LIFECYCLE_POLL_ATTEMPTS; i++) {
					observed = await observer.getOnSleepCount(actorId);
					if (observed >= 2) break;
					await waitFor(driverTestConfig, LIFECYCLE_POLL_INTERVAL_MS);
				}
				expect(observed).toBe(2);

				// onSleep also writes a sentinel row that should survive into
				// the next wake.
				let sleepEventCount = -1;
				for (let i = 0; i < LIFECYCLE_POLL_ATTEMPTS; i++) {
					sleepEventCount = await actor.getSleepEventCount();
					if (sleepEventCount >= 1) break;
					await waitFor(driverTestConfig, LIFECYCLE_POLL_INTERVAL_MS);
				}
				expect(sleepEventCount).toBe(1);
			},
			dbTestTimeout,
		);

		test(
			"onDestroy can access c.db",
			async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);
				const observer = client.dbInitOrderObserver.getOrCreate([
					"db-init-order-observer",
				]);
				const key = `db-init-order-on-destroy-${crypto.randomUUID()}`;
				const actor = client.dbInitOrderOnDestroyActor.getOrCreate([
					key,
				]);
				const actorId = await actor.getActorId();

				await actor.insert("alpha");
				await actor.insert("beta");
				await actor.insert("gamma");
				await actor.triggerDestroy();

				let observed = -1;
				for (let i = 0; i < LIFECYCLE_POLL_ATTEMPTS; i++) {
					observed = await observer.getOnDestroyCount(actorId);
					if (observed >= 3) break;
					await waitFor(driverTestConfig, LIFECYCLE_POLL_INTERVAL_MS);
				}
				expect(observed).toBe(3);
			},
			dbTestTimeout,
		);
	});
});
