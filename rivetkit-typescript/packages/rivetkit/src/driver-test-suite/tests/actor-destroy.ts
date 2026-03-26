import { describe, expect, test, vi } from "vitest";
import type { ActorError } from "@/client/mod";
import type { DriverTestConfig } from "../mod";
import { setupDriverTest } from "../utils";

export function runActorDestroyTests(driverTestConfig: DriverTestConfig) {
	describe("Actor Destroy Tests", () => {
		function expectActorNotFound(error: unknown) {
			expect((error as ActorError).group).toBe("actor");
			expect((error as ActorError).code).toBe("not_found");
		}

		async function waitForActorDestroyed(
			client: Awaited<ReturnType<typeof setupDriverTest>>["client"],
			actorKey: string,
			actorId: string,
		) {
			const observer = client.destroyObserver.getOrCreate(["observer"]);

			await vi.waitFor(async () => {
				const wasDestroyed = await observer.wasDestroyed(actorKey);
				expect(wasDestroyed, "actor onDestroy not called").toBeTruthy();
			});

			await vi.waitFor(async () => {
				let actorRunning = false;
				try {
					await client.destroyActor.getForId(actorId).getValue();
					actorRunning = true;
				} catch (error) {
					expectActorNotFound(error);
				}

				expect(actorRunning, "actor still running").toBeFalsy();
			});
		}

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
			await newHandle.resolve();

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
			await newHandle.resolve();

			// Verify state is fresh (default value, not the old value)
			const newValue = await newHandle.getValue();
			expect(newValue).toBe(0);
		});

		test("stale getOrCreate handle retries action after actor destruction", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actorKey = `test-lazy-handle-action-${crypto.randomUUID()}`;

			const observer = client.destroyObserver.getOrCreate(["observer"]);
			await observer.reset();

			const handle = client.destroyActor.getOrCreate([actorKey]);
			await handle.setValue(321);

			const originalActorId = await handle.resolve();
			await handle.destroy();
			await waitForActorDestroyed(client, actorKey, originalActorId);

			expect(await handle.getValue()).toBe(0);
		});

		test("stale getOrCreate handle retries queue send after actor destruction", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actorKey = `test-lazy-handle-queue-${crypto.randomUUID()}`;

			const observer = client.destroyObserver.getOrCreate(["observer"]);
			await observer.reset();

			const handle = client.destroyActor.getOrCreate([actorKey]);
			const originalActorId = await handle.resolve();

			await handle.destroy();
			await waitForActorDestroyed(client, actorKey, originalActorId);

			await handle.send("values", 11);
			expect(await handle.receiveValue()).toBe(11);
		});

		test("stale getOrCreate handle retries raw HTTP after actor destruction", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actorKey = `test-lazy-handle-http-${crypto.randomUUID()}`;

			const observer = client.destroyObserver.getOrCreate(["observer"]);
			await observer.reset();

			const handle = client.destroyActor.getOrCreate([actorKey]);
			await handle.setValue(55);

			const originalActorId = await handle.resolve();
			await handle.destroy();
			await waitForActorDestroyed(client, actorKey, originalActorId);

			const response = await handle.fetch("/state");
			expect(response.ok).toBe(true);
			expect(await response.json()).toEqual({
				key: actorKey,
				value: 0,
			});
		});

		test("stale getOrCreate handle retries raw WebSocket after actor destruction", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actorKey = `test-lazy-handle-websocket-${crypto.randomUUID()}`;

			const observer = client.destroyObserver.getOrCreate(["observer"]);
			await observer.reset();

			const handle = client.destroyActor.getOrCreate([actorKey]);
			await handle.setValue(89);

			const originalActorId = await handle.resolve();
			await handle.destroy();
			await waitForActorDestroyed(client, actorKey, originalActorId);

			const websocket = await handle.webSocket();
			const welcome = await new Promise<{
				type: string;
				key: string;
				value: number;
			}>((resolve, reject) => {
				websocket.addEventListener(
					"message",
					(event: MessageEvent) => {
						resolve(JSON.parse(event.data));
					},
					{ once: true },
				);
				websocket.addEventListener("close", reject, { once: true });
			});
			expect(welcome).toEqual({
				type: "welcome",
				key: actorKey,
				value: 0,
			});
			websocket.close();
		});

		test("stale getOrCreate connection re-resolves after websocket open failure", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actorKey = `test-lazy-handle-connect-${crypto.randomUUID()}`;

			const observer = client.destroyObserver.getOrCreate(["observer"]);
			await observer.reset();

			const handle = client.destroyActor.getOrCreate([actorKey]);
			await handle.setValue(144);

			const originalActorId = await handle.resolve();
			await handle.destroy();
			await waitForActorDestroyed(client, actorKey, originalActorId);

			const connection = handle.connect();
			expect(await connection.getValue()).toBe(0);
			await connection.dispose();
		});

		test("stale get handle retries action after actor recreation", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actorKey = `test-lazy-get-handle-action-${crypto.randomUUID()}`;

			const observer = client.destroyObserver.getOrCreate(["observer"]);
			await observer.reset();

			const creator = client.destroyActor.getOrCreate([actorKey]);
			await creator.setValue(222);

			const handle = client.destroyActor.get([actorKey]);
			expect(await handle.getValue()).toBe(222);

			const originalActorId = await creator.resolve();
			await creator.destroy();
			await waitForActorDestroyed(client, actorKey, originalActorId);

			const recreated = client.destroyActor.getOrCreate([actorKey]);
			expect(await recreated.getValue()).toBe(0);
			expect(await handle.getValue()).toBe(0);
			expect(await handle.resolve()).toBe(await recreated.resolve());
		});
	});
}
