import { describe, expect, test } from "vitest";
import type { DriverTestConfig } from "../mod";
import { setupDriverTest } from "../utils";

export function runActorStatelessTests(driverTestConfig: DriverTestConfig) {
	describe("Actor Stateless Tests", () => {
		describe("Stateless Actor Operations", () => {
			test("can call actions on stateless actor", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				const instance = client.statelessActor.getOrCreate();

				const result = await instance.ping();
				expect(result).toBe("pong");
			});

			test("can echo messages", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				const instance = client.statelessActor.getOrCreate();

				const message = "Hello, World!";
				const result = await instance.echo(message);
				expect(result).toBe(message);
			});

			test("can access actorId", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				const instance = client.statelessActor.getOrCreate(["test-id"]);

				const actorId = await instance.getActorId();
				expect(actorId).toBeDefined();
				expect(typeof actorId).toBe("string");
			});

			test("accessing state throws StateNotEnabled", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				const instance = client.statelessActor.getOrCreate();

				const result = await instance.tryGetState();
				expect(result.success).toBe(false);
				expect(result.error).toContain("state");
			});

			test("accessing db throws DatabaseNotEnabled", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				const instance = client.statelessActor.getOrCreate();

				const result = await instance.tryGetDb();
				expect(result.success).toBe(false);
				expect(result.error).toContain("database");
			});

			test("multiple stateless actors can exist independently", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				const actor1 = client.statelessActor.getOrCreate(["actor-1"]);
				const actor2 = client.statelessActor.getOrCreate(["actor-2"]);

				const id1 = await actor1.getActorId();
				const id2 = await actor2.getActorId();

				expect(id1).not.toBe(id2);
			});
		});
	});
}
