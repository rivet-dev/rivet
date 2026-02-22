import { describe, expect, test } from "vitest";
import type { DriverTestConfig } from "../mod";
import { setupDriverTest, waitFor } from "../utils";

type DbVariant = "raw" | "drizzle";

const CHUNK_SIZE = 4096;
const LARGE_PAYLOAD_SIZE = 32768;
const HIGH_VOLUME_COUNT = 1000;
const SLEEP_WAIT_MS = 150;

function getDbActor(
	client: Awaited<ReturnType<typeof setupDriverTest>>["client"],
	variant: DbVariant,
) {
	return variant === "raw" ? client.dbActorRaw : client.dbActorDrizzle;
}

export function runActorDbTests(driverTestConfig: DriverTestConfig) {
	const variants: DbVariant[] = ["raw", "drizzle"];

	for (const variant of variants) {
		describe(`Actor Database (${variant}) Tests`, () => {
			test("bootstraps schema on startup", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);
				const actor = getDbActor(client, variant).getOrCreate([
					`db-${variant}-bootstrap-${crypto.randomUUID()}`,
				]);

				const count = await actor.getCount();
				expect(count).toBe(0);
			});

			test("supports CRUD, raw SQL, and multi-statement exec", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);
				const actor = getDbActor(client, variant).getOrCreate([
					`db-${variant}-crud-${crypto.randomUUID()}`,
				]);

				await actor.reset();

				const first = await actor.insertValue("alpha");
				const second = await actor.insertValue("beta");

				const values = await actor.getValues();
				expect(values).toHaveLength(2);
				expect(values[0].value).toBe("alpha");
				expect(values[1].value).toBe("beta");

				await actor.updateValue(first.id, "alpha-updated");
				const updated = await actor.getValue(first.id);
				expect(updated).toBe("alpha-updated");

				await actor.deleteValue(second.id);
				const count = await actor.getCount();
				expect(count).toBe(1);

				const rawCount = await actor.rawSelectCount();
				expect(rawCount).toBe(1);

				const multiValue = await actor.multiStatementInsert("gamma");
				expect(multiValue).toBe("gamma-updated");
			});

			test("handles transactions", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);
				const actor = getDbActor(client, variant).getOrCreate([
					`db-${variant}-tx-${crypto.randomUUID()}`,
				]);

				await actor.reset();
				await actor.transactionCommit("commit");
				expect(await actor.getCount()).toBe(1);

				await actor.transactionRollback("rollback");
				expect(await actor.getCount()).toBe(1);
			});

			test("persists across sleep and wake cycles", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);
				const actor = getDbActor(client, variant).getOrCreate([
					`db-${variant}-sleep-${crypto.randomUUID()}`,
				]);

				await actor.reset();
				await actor.insertValue("sleepy");
				expect(await actor.getCount()).toBe(1);

				for (let i = 0; i < 3; i++) {
					await actor.triggerSleep();
					await waitFor(driverTestConfig, SLEEP_WAIT_MS);
					expect(await actor.getCount()).toBe(1);
				}
			});

			test("completes onDisconnect DB writes before sleeping", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);
				const key = `db-${variant}-disconnect-${crypto.randomUUID()}`;

				const actor = getDbActor(client, variant).getOrCreate([key]);
				await actor.reset();
				await actor.configureDisconnectInsert(true, 250);

				await waitFor(driverTestConfig, SLEEP_WAIT_MS + 250);
				await actor.configureDisconnectInsert(false, 0);

				expect(await actor.getDisconnectInsertCount()).toBe(1);
			});

			test("handles high-volume inserts", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);
				const actor = getDbActor(client, variant).getOrCreate([
					`db-${variant}-high-volume-${crypto.randomUUID()}`,
				]);

				await actor.reset();
				await actor.insertMany(HIGH_VOLUME_COUNT);
				expect(await actor.getCount()).toBe(HIGH_VOLUME_COUNT);
			});

			test("handles payloads across chunk boundaries", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);
				const actor = getDbActor(client, variant).getOrCreate([
					`db-${variant}-chunk-${crypto.randomUUID()}`,
				]);

				await actor.reset();
				const sizes = [CHUNK_SIZE - 1, CHUNK_SIZE, CHUNK_SIZE + 1];
				for (const size of sizes) {
					const { id } = await actor.insertPayloadOfSize(size);
					const storedSize = await actor.getPayloadSize(id);
					expect(storedSize).toBe(size);
				}
			});

			test("handles large payloads", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);
				const actor = getDbActor(client, variant).getOrCreate([
					`db-${variant}-large-${crypto.randomUUID()}`,
				]);

				await actor.reset();
				const { id } = await actor.insertPayloadOfSize(LARGE_PAYLOAD_SIZE);
				const storedSize = await actor.getPayloadSize(id);
				expect(storedSize).toBe(LARGE_PAYLOAD_SIZE);
			});

			test("handles repeated updates to the same row", async (c) => {
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
			});
		});
	}
}
