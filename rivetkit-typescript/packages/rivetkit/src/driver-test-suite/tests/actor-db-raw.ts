import { describe, expect, test } from "vitest";
import type { DriverTestConfig } from "../mod";
import { setupDriverTest } from "../utils";

export function runActorDbRawTests(driverTestConfig: DriverTestConfig) {
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
				const instance1 = client.dbActorRaw.getOrCreate(["test-persistence"]);
				await instance1.insertValue("Item 1");
				await instance1.insertValue("Item 2");

				// Second instance (same actor) should see persisted data
				const instance2 = client.dbActorRaw.getOrCreate(["test-persistence"]);
				const count = await instance2.getCount();
				expect(count).toBe(2);
			});

			test("maintains separate databases for different actors", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				// First actor
				const actor1 = client.dbActorRaw.getOrCreate(["actor-1"]);
				await actor1.insertValue("A");
				await actor1.insertValue("B");

				// Second actor
				const actor2 = client.dbActorRaw.getOrCreate(["actor-2"]);
				await actor2.insertValue("X");

				// Verify separate data
				const count1 = await actor1.getCount();
				const count2 = await actor2.getCount();
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
}
