import { describe, expect, test } from "vitest";
import type { DriverTestConfig } from "../mod";
import { setupDriverTest, waitFor } from "../utils";

const SLEEP_WAIT_MS = 150;

export function runDynamicSqliteProxyTests(
	driverTestConfig: DriverTestConfig,
) {
	describe.skipIf(!driverTestConfig.isDynamic)(
		"Dynamic Actor SQLite Proxy Tests",
		() => {
			describe("Raw db() through SQLite proxy", () => {
				test("creates tables via onMigrate, inserts and queries rows", async (c) => {
					const { client } = await setupDriverTest(
						c,
						driverTestConfig,
					);
					const actor = client.dbActorRaw.getOrCreate([
						`proxy-raw-crud-${crypto.randomUUID()}`,
					]);

					await actor.reset();

					const { id: id1 } = await actor.insertValue("alice");
					const { id: id2 } = await actor.insertValue("bob");

					const values = await actor.getValues();
					expect(values).toHaveLength(2);
					expect(values[0].value).toBe("alice");
					expect(values[1].value).toBe("bob");

					const single = await actor.getValue(id1);
					expect(single).toBe("alice");

					await actor.updateValue(id2, "bob-updated");
					expect(await actor.getValue(id2)).toBe("bob-updated");

					await actor.deleteValue(id1);
					expect(await actor.getCount()).toBe(1);
				});

				test("persists data across sleep/wake cycles", async (c) => {
					const { client } = await setupDriverTest(
						c,
						driverTestConfig,
					);
					const actor = client.dbActorRaw.getOrCreate([
						`proxy-raw-sleep-${crypto.randomUUID()}`,
					]);

					await actor.reset();
					await actor.insertValue("before-sleep-1");
					await actor.insertValue("before-sleep-2");
					expect(await actor.getCount()).toBe(2);

					// Sleep and wake
					await actor.triggerSleep();
					await waitFor(driverTestConfig, SLEEP_WAIT_MS);

					// After wake, data should persist
					expect(await actor.getCount()).toBe(2);
					const values = await actor.getValues();
					expect(
						values.some(
							(r: { value: string }) =>
								r.value === "before-sleep-1",
						),
					).toBe(true);

					// Insert after wake and sleep again
					await actor.insertValue("after-wake");
					expect(await actor.getCount()).toBe(3);

					await actor.triggerSleep();
					await waitFor(driverTestConfig, SLEEP_WAIT_MS);

					expect(await actor.getCount()).toBe(3);
				});

				test("handles transactions through proxy", async (c) => {
					const { client } = await setupDriverTest(
						c,
						driverTestConfig,
					);
					const actor = client.dbActorRaw.getOrCreate([
						`proxy-raw-tx-${crypto.randomUUID()}`,
					]);

					await actor.reset();

					await actor.transactionCommit("committed");
					expect(await actor.getCount()).toBe(1);

					await actor.transactionRollback("rolled-back");
					expect(await actor.getCount()).toBe(1);
				});
			});

			describe("Drizzle db() through SQLite proxy", () => {
				test("runs drizzle migrations and queries through proxy", async (c) => {
					const { client } = await setupDriverTest(
						c,
						driverTestConfig,
					);
					const actor = client.dbActorDrizzle.getOrCreate([
						`proxy-drizzle-crud-${crypto.randomUUID()}`,
					]);

					await actor.reset();

					const { id } = await actor.insertValue("drizzle-test");
					expect(await actor.getValue(id)).toBe("drizzle-test");

					await actor.insertValue("drizzle-test-2");
					expect(await actor.getCount()).toBe(2);

					const values = await actor.getValues();
					expect(values).toHaveLength(2);
				});

				test("persists drizzle data across sleep/wake cycles", async (c) => {
					const { client } = await setupDriverTest(
						c,
						driverTestConfig,
					);
					const actor = client.dbActorDrizzle.getOrCreate([
						`proxy-drizzle-sleep-${crypto.randomUUID()}`,
					]);

					await actor.reset();
					await actor.insertValue("drizzle-sleep-1");
					expect(await actor.getCount()).toBe(1);

					await actor.triggerSleep();
					await waitFor(driverTestConfig, SLEEP_WAIT_MS);

					expect(await actor.getCount()).toBe(1);
					expect(
						(await actor.getValues())[0].value,
					).toBe("drizzle-sleep-1");
				});

				test("handles multi-statement inserts through proxy", async (c) => {
					const { client } = await setupDriverTest(
						c,
						driverTestConfig,
					);
					const actor = client.dbActorDrizzle.getOrCreate([
						`proxy-drizzle-multi-${crypto.randomUUID()}`,
					]);

					await actor.reset();

					const result =
						await actor.multiStatementInsert("multi-drizzle");
					expect(result).toBe("multi-drizzle-updated");
				});
			});
		},
	);
}
