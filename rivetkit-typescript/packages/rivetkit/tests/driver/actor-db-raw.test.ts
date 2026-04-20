import { describeDriverMatrix } from "./shared-matrix";
import { describe, expect, test, vi } from "vitest";
import { setupDriverTest } from "./shared-utils";

const DB_READY_TIMEOUT_MS = 10_000;

describeDriverMatrix("Actor Db Raw", (driverTestConfig) => {
	describe("Actor Database (Raw) Tests", () => {
		describe("Database Basic Operations", () => {
			test("creates and queries database tables", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				const instance = client.dbActorRaw.getOrCreate();

				// Add values
				await instance.insertValue("Alice");
				await instance.insertValue("Bob");

				// Query values
				const values = await instance.getValues();
				expect(values).toHaveLength(2);
				expect(values[0].value).toBe("Alice");
				expect(values[1].value).toBe("Bob");
			});

			test("persists data across actor instances", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				// First instance adds items
				const instance1 = client.dbActorRaw.getOrCreate([
					"test-persistence",
				]);
				await instance1.insertValue("Item 1");
				await instance1.insertValue("Item 2");

				// Second instance (same actor) should see persisted data
				const instance2 = client.dbActorRaw.getOrCreate([
					"test-persistence",
				]);
				const count = await instance2.getCount();
				expect(count).toBe(2);
			});

			test("maintains separate databases for different actors", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);
				const actor1Key = ["actor-1"];
				const actor2Key = ["actor-2"];
				const getActor1 = () => client.dbActorRaw.getOrCreate(actor1Key);
				const getActor2 = () => client.dbActorRaw.getOrCreate(actor2Key);

				// First actor
				await getActor1().insertValue("A");
				await getActor1().insertValue("B");

				// Second actor
				await getActor2().insertValue("X");

				// Reacquire keyed handles after the writes; fast sleep can leave
				// older direct targets pointing at a stopping actor instance.
				await vi.waitFor(
						async () => {
							const count1 = await getActor1().getCount();
							const count2 = await getActor2().getCount();
							expect(count1).toBe(2);
							expect(count2).toBe(1);
						},
					{ timeout: DB_READY_TIMEOUT_MS, interval: 100 },
				);
			});
		});

		describe("Database Migrations", () => {
			test("runs migrations on actor startup", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				const instance = client.dbActorRaw.getOrCreate();

				// Try to insert into the table to verify it exists
				await instance.insertValue("test");
				const values = await instance.getValues();

				expect(values).toHaveLength(1);
				expect(values[0].value).toBe("test");
			});
		});
	});
});
