import { describe, expect, test } from "vitest";
import { describeDriverMatrix } from "./shared-matrix";
import { setupDriverTest } from "./shared-utils";

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

			test("round-trips text containing an embedded nul byte", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				const instance = client.dbActorRaw.getOrCreate(["nul-text"]);
				const input = "a\0b";

				const row = await instance.insertValueAndReadBack(input);

				expect(row).not.toBeNull();
				expect(row?.value).toBe(input);
				expect(row?.hex_value).toBe("610062");
				expect(row?.sqlite_length).toBe(1);
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
				const getActor1 = () =>
					client.dbActorRaw.getOrCreate(actor1Key);
				const getActor2 = () =>
					client.dbActorRaw.getOrCreate(actor2Key);

				// First actor
				await getActor1().insertValue("A");
				await getActor1().insertValue("B");

				// Second actor
				await getActor2().insertValue("X");

				const verifyActor1 = getActor1();
				const verifyActor2 = getActor2();
				await Promise.all([verifyActor1.ready, verifyActor2.ready]);
				const count1 = await verifyActor1.getCount();
				const count2 = await verifyActor2.getCount();
				expect(count1).toBe(2);
				expect(count2).toBe(1);
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
