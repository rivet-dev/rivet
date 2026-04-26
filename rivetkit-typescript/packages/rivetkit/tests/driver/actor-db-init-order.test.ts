import { describe, expect, test } from "vitest";
import { describeDriverMatrix } from "./shared-matrix";
import { setupDriverTest } from "./shared-utils";

const REAL_TIMER_DB_TIMEOUT_MS = 180_000;

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

				await actor.insert("alpha");
				const nextActor = client.dbInitOrderCreateStateActor.getOrCreate(
					[key],
				);
				expect(typeof (await nextActor.getInitialCount())).toBe("number");
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
});
