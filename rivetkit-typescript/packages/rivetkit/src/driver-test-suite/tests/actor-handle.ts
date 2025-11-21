import { describe, expect, test } from "vitest";
import type { ActorError } from "@/client/mod";
import type { DriverTestConfig } from "../mod";
import { setupDriverTest } from "../utils";

export function runActorHandleTests(driverTestConfig: DriverTestConfig) {
	describe("Actor Handle Tests", () => {
		describe("Access Methods", () => {
			test("should use .get() to access a actor", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				// Create actor first
				await client.counter.create(["test-get-handle"]);

				// Access using get
				const handle = client.counter.get(["test-get-handle"]);

				// Verify Action works
				const count = await handle.increment(5);
				expect(count).toBe(5);

				const retrievedCount = await handle.getCount();
				expect(retrievedCount).toBe(5);
			});

			test("should use .getForId() to access a actor by ID", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				// Create a actor first to get its ID
				const handle = client.counter.getOrCreate([
					"test-get-for-id-handle",
				]);
				await handle.increment(3);
				const actorId = await handle.resolve();

				// Access using getForId
				const idHandle = client.counter.getForId(actorId);

				// Verify Action works and state is preserved
				const count = await idHandle.getCount();
				expect(count).toBe(3);

				const newCount = await idHandle.increment(4);
				expect(newCount).toBe(7);
			});

			test("should use .getOrCreate() to access or create a actor", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				// Access using getOrCreate - should create the actor
				const handle = client.counter.getOrCreate([
					"test-get-or-create-handle",
				]);

				// Verify Action works
				const count = await handle.increment(7);
				expect(count).toBe(7);

				// Get the same actor again - should retrieve existing actor
				const sameHandle = client.counter.getOrCreate([
					"test-get-or-create-handle",
				]);
				const retrievedCount = await sameHandle.getCount();
				expect(retrievedCount).toBe(7);
			});

			test("should use (await create()) to create and return a handle", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				// Create actor and get handle
				const handle = await client.counter.create([
					"test-create-handle",
				]);

				// Verify Action works
				const count = await handle.increment(9);
				expect(count).toBe(9);

				const retrievedCount = await handle.getCount();
				expect(retrievedCount).toBe(9);
			});

			test("errors when calling create twice with the same key", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				const key = ["duplicate-create-handle", crypto.randomUUID()];

				// First create should succeed
				await client.counter.create(key);

				// Second create with same key should throw ActorAlreadyExists
				try {
					await client.counter.create(key);
					expect.fail("did not error on duplicate create");
				} catch (err) {
					expect((err as ActorError).group).toBe("actor");
					expect((err as ActorError).code).toBe("duplicate_key");
				}
			});

			test(".get().resolve() errors for non-existent actor", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				const missingId = `nonexistent-${crypto.randomUUID()}`;

				try {
					await client.counter.get([missingId]).resolve();
					expect.fail(
						"did not error for get().resolve() on missing actor",
					);
				} catch (err) {
					expect((err as ActorError).group).toBe("actor");
					expect((err as ActorError).code).toBe("not_found");
				}
			});
		});

		describe("Action Functionality", () => {
			test("should call actions directly on the handle", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				const handle = client.counter.getOrCreate([
					"test-action-handle",
				]);

				// Call multiple actions in sequence
				const count1 = await handle.increment(3);
				expect(count1).toBe(3);

				const count2 = await handle.increment(5);
				expect(count2).toBe(8);

				const retrievedCount = await handle.getCount();
				expect(retrievedCount).toBe(8);
			});

			test("should handle independent handles to the same actor", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				// Create two handles to the same actor
				const handle1 = client.counter.getOrCreate([
					"test-multiple-handles",
				]);
				const handle2 = client.counter.get(["test-multiple-handles"]);

				// Call actions on both handles
				await handle1.increment(3);
				const count = await handle2.getCount();

				// Verify both handles access the same state
				expect(count).toBe(3);

				const finalCount = await handle2.increment(4);
				expect(finalCount).toBe(7);

				const checkCount = await handle1.getCount();
				expect(checkCount).toBe(7);
			});

			test("should resolve a actor's ID", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				const handle = client.counter.getOrCreate(["test-resolve-id"]);

				// Call an action to ensure actor exists
				await handle.increment(1);

				// Resolve the ID
				const actorId = await handle.resolve();

				// Verify we got a valid ID (string)
				expect(typeof actorId).toBe("string");
				expect(actorId).not.toBe("");

				// Verify we can use this ID to get the actor
				const idHandle = client.counter.getForId(actorId);
				const count = await idHandle.getCount();
				expect(count).toBe(1);
			});
		});

		describe("Lifecycle Hooks", () => {
			test("should trigger lifecycle hooks on actor creation", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				// Get or create a new actor - this should trigger onWake
				const handle = client.counterWithLifecycle.getOrCreate([
					"test-lifecycle-handle",
				]);

				// Verify onWake was triggered
				const initialEvents = await handle.getEvents();
				expect(initialEvents).toContain("onWake");

				// Create a separate handle to the same actor
				const sameHandle = client.counterWithLifecycle.getOrCreate([
					"test-lifecycle-handle",
				]);

				// Verify events still include onWake but don't duplicate it
				// (onWake should only be called once when the actor is first created)
				const events = await sameHandle.getEvents();
				expect(events).toContain("onWake");
				expect(events.filter((e) => e === "onWake").length).toBe(1);
			});

			test("should trigger lifecycle hooks for each Action call", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				// Create a normal handle to view events
				const viewHandle = client.counterWithLifecycle.getOrCreate([
					"test-lifecycle-action",
				]);

				// Initial state should only have onWake
				const initialEvents = await viewHandle.getEvents();
				expect(initialEvents).toContain("onWake");
				expect(initialEvents).not.toContain("onBeforeConnect");
				expect(initialEvents).not.toContain("onConnect");
				expect(initialEvents).not.toContain("onDisconnect");

				// Create a handle with trackLifecycle enabled for testing Action calls
				const trackingHandle = client.counterWithLifecycle.getOrCreate(
					["test-lifecycle-action"],
					{ params: { trackLifecycle: true } },
				);

				// Make an Action call
				await trackingHandle.increment(5);

				// Check that it triggered the lifecycle hooks
				const eventsAfterAction = await viewHandle.getEvents();

				// Should have onBeforeConnect, onConnect, and onDisconnect for the Action call
				expect(eventsAfterAction).toContain("onBeforeConnect");
				expect(eventsAfterAction).toContain("onConnect");
				expect(eventsAfterAction).toContain("onDisconnect");

				// Each should have count 1
				expect(
					eventsAfterAction.filter((e) => e === "onBeforeConnect")
						.length,
				).toBe(1);
				expect(
					eventsAfterAction.filter((e) => e === "onConnect").length,
				).toBe(1);
				expect(
					eventsAfterAction.filter((e) => e === "onDisconnect")
						.length,
				).toBe(1);

				// Make another Action call
				await trackingHandle.increment(10);

				// Check that it triggered another set of lifecycle hooks
				const eventsAfterSecondAction = await viewHandle.getEvents();

				// Each hook should now have count 2
				expect(
					eventsAfterSecondAction.filter(
						(e) => e === "onBeforeConnect",
					).length,
				).toBe(2);
				expect(
					eventsAfterSecondAction.filter((e) => e === "onConnect")
						.length,
				).toBe(2);
				expect(
					eventsAfterSecondAction.filter((e) => e === "onDisconnect")
						.length,
				).toBe(2);
			});

			test("should trigger lifecycle hooks for each Action call across multiple handles", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				// Create a normal handle to view events
				const viewHandle = client.counterWithLifecycle.getOrCreate([
					"test-lifecycle-multi-handle",
				]);

				// Create two tracking handles to the same actor
				const trackingHandle1 = client.counterWithLifecycle.getOrCreate(
					["test-lifecycle-multi-handle"],
					{ params: { trackLifecycle: true } },
				);

				const trackingHandle2 = client.counterWithLifecycle.getOrCreate(
					["test-lifecycle-multi-handle"],
					{ params: { trackLifecycle: true } },
				);

				// Make Action calls on both handles
				await trackingHandle1.increment(5);
				await trackingHandle2.increment(10);

				// Check lifecycle hooks
				const events = await viewHandle.getEvents();

				// Should have 1 onWake, 2 each of onBeforeConnect, onConnect, and onDisconnect
				expect(events.filter((e) => e === "onWake").length).toBe(1);
				expect(
					events.filter((e) => e === "onBeforeConnect").length,
				).toBe(2);
				expect(events.filter((e) => e === "onConnect").length).toBe(2);
				expect(events.filter((e) => e === "onDisconnect").length).toBe(
					2,
				);
			});
		});
	});
}
