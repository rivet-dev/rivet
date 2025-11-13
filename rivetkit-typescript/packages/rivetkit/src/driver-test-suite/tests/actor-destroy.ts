import { describe, expect, test, vi } from "vitest";
import type { ActorError } from "@/client/mod";
import type { DriverTestConfig } from "../mod";
import { setupDriverTest } from "../utils";

export function runActorDestroyTests(driverTestConfig: DriverTestConfig) {
	describe("Actor Destroy Tests", () => {
		test("actor destroy clears state (without connect)", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);

			const actorKey = "test-destroy-without-connect";

			// Get destroy observer
			const observer = client.destroyObserver.getOrCreate(["observer"]);
			await observer.reset();

			// Create actor
			const destroyActor = client.destroyActor.getOrCreate([actorKey]);

			// Update state and save immediately
			await destroyActor.setValue(42);

			// Verify state was saved
			const value = await destroyActor.getValue();
			expect(value).toBe(42);

			// Get actor ID before destroying
			const actorId = await destroyActor.resolve();

			// Destroy the actor
			await destroyActor.destroy();

			// Wait until the observer confirms the actor was destroyed
			await vi.waitFor(async () => {
				const wasDestroyed = await observer.wasDestroyed(actorKey);
				expect(wasDestroyed, "actor onDestroy not called").toBeTruthy();
			});

			// Wait until the actor is fully cleaned up (getForId returns error)
			await vi.waitFor(async () => {
				let actorRunning = false;
				try {
					await client.destroyActor.getForId(actorId).getValue();
					actorRunning = true;
				} catch (err) {
					expect((err as ActorError).group).toBe("actor");
					expect((err as ActorError).code).toBe("not_found");
				}

				expect(actorRunning, "actor still running").toBeFalsy();
			});

			// Verify actor no longer exists via getForId
			let existsById = false;
			try {
				await client.destroyActor.getForId(actorId).getValue();
				existsById = true;
			} catch (err) {
				expect((err as ActorError).group).toBe("actor");
				expect((err as ActorError).code).toBe("not_found");
			}
			expect(
				existsById,
				"actor should not exist after destroy",
			).toBeFalsy();

			// Verify actor no longer exists via get
			let existsByKey = false;
			try {
				await client.destroyActor
					.get(["test-destroy-without-connect"])
					.resolve();
				existsByKey = true;
			} catch (err) {
				expect((err as ActorError).group).toBe("actor");
				expect((err as ActorError).code).toBe("not_found");
			}
			expect(
				existsByKey,
				"actor should not exist after destroy",
			).toBeFalsy();

			// Create new actor with same key using getOrCreate
			const newActor = client.destroyActor.getOrCreate([
				"test-destroy-without-connect",
			]);

			// Verify state is fresh (default value, not the old value)
			const newValue = await newActor.getValue();
			expect(newValue).toBe(0);
		});

		test("actor destroy clears state (with connect)", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);

			const actorKey = "test-destroy-with-connect";

			// Get destroy observer
			const observer = client.destroyObserver.getOrCreate(["observer"]);
			await observer.reset();

			// Create actor handle
			const destroyActorHandle = client.destroyActor.getOrCreate([
				actorKey,
			]);

			// Get actor ID before destroying
			const actorId = await destroyActorHandle.resolve();

			// Create persistent connection
			const destroyActor = destroyActorHandle.connect();

			// Update state and save immediately
			await destroyActor.setValue(99);

			// Verify state was saved
			const value = await destroyActor.getValue();
			expect(value).toBe(99);

			// Destroy the actor
			await destroyActor.destroy();

			// Dispose the connection
			await destroyActor.dispose();

			// Wait until the observer confirms the actor was destroyed
			await vi.waitFor(async () => {
				const wasDestroyed = await observer.wasDestroyed(actorKey);
				expect(wasDestroyed, "actor onDestroy not called").toBeTruthy();
			});

			// Wait until the actor is fully cleaned up (getForId returns error)
			await vi.waitFor(async () => {
				let actorRunning = false;
				try {
					await client.destroyActor.getForId(actorId).getValue();
					actorRunning = true;
				} catch (err) {
					expect((err as ActorError).group).toBe("actor");
					expect((err as ActorError).code).toBe("not_found");
				}

				expect(actorRunning, "actor still running").toBeFalsy();
			});

			// Verify actor no longer exists via getForId
			let existsById = false;
			try {
				await client.destroyActor.getForId(actorId).getValue();
				existsById = true;
			} catch (err) {
				expect((err as ActorError).group).toBe("actor");
				expect((err as ActorError).code).toBe("not_found");
			}
			expect(
				existsById,
				"actor should not exist after destroy",
			).toBeFalsy();

			// Verify actor no longer exists via get
			let existsByKey = false;
			try {
				await client.destroyActor
					.get(["test-destroy-with-connect"])
					.resolve();
				existsByKey = true;
			} catch (err) {
				expect((err as ActorError).group).toBe("actor");
				expect((err as ActorError).code).toBe("not_found");
			}
			expect(
				existsByKey,
				"actor should not exist after destroy",
			).toBeFalsy();

			// Create new actor with same key using getOrCreate
			const newActor = client.destroyActor.getOrCreate([
				"test-destroy-with-connect",
			]);

			// Verify state is fresh (default value, not the old value)
			const newValue = await newActor.getValue();
			expect(newValue).toBe(0);
		});

		test("actor destroy allows recreation via getOrCreate with resolve", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);

			const actorKey = "test-destroy-getorcreate-resolve";

			// Get destroy observer
			const observer = client.destroyObserver.getOrCreate(["observer"]);
			await observer.reset();

			// Create actor
			const destroyActor = client.destroyActor.getOrCreate([actorKey]);

			// Update state and save immediately
			await destroyActor.setValue(123);

			// Verify state was saved
			const value = await destroyActor.getValue();
			expect(value).toBe(123);

			// Get actor ID before destroying
			const actorId = await destroyActor.resolve();

			// Destroy the actor
			await destroyActor.destroy();

			// Wait until the observer confirms the actor was destroyed
			await vi.waitFor(async () => {
				const wasDestroyed = await observer.wasDestroyed(actorKey);
				expect(wasDestroyed, "actor onDestroy not called").toBeTruthy();
			});

			// Wait until the actor is fully cleaned up
			await vi.waitFor(async () => {
				let actorRunning = false;
				try {
					await client.destroyActor.getForId(actorId).getValue();
					actorRunning = true;
				} catch (err) {
					expect((err as ActorError).group).toBe("actor");
					expect((err as ActorError).code).toBe("not_found");
				}

				expect(actorRunning, "actor still running").toBeFalsy();
			});

			// Recreate using getOrCreate with resolve
			const newHandle = client.destroyActor.getOrCreate([actorKey]);
			const newActorId = await newHandle.resolve();

			// Verify state is fresh (default value, not the old value)
			const newValue = await newHandle.getValue();
			expect(newValue).toBe(0);
		});

		test("actor destroy allows recreation via create", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);

			const actorKey = "test-destroy-create";

			// Get destroy observer
			const observer = client.destroyObserver.getOrCreate(["observer"]);
			await observer.reset();

			// Create actor using create()
			const initialHandle = await client.destroyActor.create([actorKey]);

			// Update state and save immediately
			await initialHandle.setValue(456);

			// Verify state was saved
			const value = await initialHandle.getValue();
			expect(value).toBe(456);

			// Get actor ID before destroying
			const actorId = await initialHandle.resolve();

			// Destroy the actor
			await initialHandle.destroy();

			// Wait until the observer confirms the actor was destroyed
			await vi.waitFor(async () => {
				const wasDestroyed = await observer.wasDestroyed(actorKey);
				expect(wasDestroyed, "actor onDestroy not called").toBeTruthy();
			});

			// Wait until the actor is fully cleaned up
			await vi.waitFor(async () => {
				let actorRunning = false;
				try {
					await client.destroyActor.getForId(actorId).getValue();
					actorRunning = true;
				} catch (err) {
					expect((err as ActorError).group).toBe("actor");
					expect((err as ActorError).code).toBe("not_found");
				}

				expect(actorRunning, "actor still running").toBeFalsy();
			});

			// Recreate using create()
			const newHandle = await client.destroyActor.create([actorKey]);
			const newActorId = await newHandle.resolve();

			// Verify state is fresh (default value, not the old value)
			const newValue = await newHandle.getValue();
			expect(newValue).toBe(0);
		});
	});
}
