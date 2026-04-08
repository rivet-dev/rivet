import { describe, expect, test } from "vitest";
import type { DriverTestConfig } from "../mod";
import { setupDriverTest } from "../utils";

export function runActorVarsTests(driverTestConfig: DriverTestConfig) {
	describe("Actor Variables", () => {
		describe("Static vars", () => {
			test("should provide access to static vars", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				const instance = client.staticVarActor.getOrCreate();

				// Test accessing vars
				const result = await instance.getVars();
				expect(result).toEqual({ counter: 42, name: "test-actor" });

				// Test accessing specific var property
				const name = await instance.getName();
				expect(name).toBe("test-actor");
			});
		});

		describe("Deep cloning of static vars", () => {
			test("should deep clone static vars between actor instances", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				// Create two separate instances
				const instance1 = client.nestedVarActor.getOrCreate([
					"instance1",
				]);
				const instance2 = client.nestedVarActor.getOrCreate([
					"instance2",
				]);

				// Modify vars in the first instance
				const modifiedVars = await instance1.modifyNested();
				expect(modifiedVars.nested.value).toBe("modified");
				expect(modifiedVars.nested.array).toContain(4);
				expect(modifiedVars.nested.obj.key).toBe("new-value");

				// Check that the second instance still has the original values
				const instance2Vars = await instance2.getVars();
				expect(instance2Vars.nested.value).toBe("original");
				expect(instance2Vars.nested.array).toEqual([1, 2, 3]);
				expect(instance2Vars.nested.obj.key).toBe("value");
			});
		});

		describe("createVars", () => {
			test("should support dynamic vars creation", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				// Create an instance
				const instance = client.dynamicVarActor.getOrCreate();

				// Test accessing dynamically created vars
				const vars = await instance.getVars();
				expect(vars).toHaveProperty("random");
				expect(vars).toHaveProperty("computed");
				expect(typeof vars.random).toBe("number");
				expect(typeof vars.computed).toBe("string");
				expect(vars.computed).toMatch(/^Actor-\d+$/);
			});

			test("should create different vars for different instances", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				// Create two separate instances
				const instance1 = client.uniqueVarActor.getOrCreate(["test1"]);
				const instance2 = client.uniqueVarActor.getOrCreate(["test2"]);

				// Get vars from both instances
				const vars1 = await instance1.getVars();
				const vars2 = await instance2.getVars();

				// Verify they have different values
				expect(vars1.id).not.toBe(vars2.id);
			});
		});

		describe("Driver Context", () => {
			test("should provide access to driver context", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				// Create an instance
				const instance = client.driverCtxActor.getOrCreate();

				// Test accessing driver context through vars
				const vars = await instance.getVars();

				// Driver context might or might not be available depending on the driver
				// But the test should run without errors
				expect(vars).toHaveProperty("hasDriverCtx");
			});
		});
	});
}
