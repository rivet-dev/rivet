import { describe, expect, test, vi } from "vitest";
import type { DriverTestConfig } from "../mod";
import { FAKE_TIME, setupDriverTest, waitFor } from "../utils";

export function runActorConnTests(driverTestConfig: DriverTestConfig) {
	describe("Actor Connection Tests", () => {
		describe("Connection Methods", () => {
			test("should connect using .get().connect()", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				// Create actor
				await client.counter.create(["test-get"]);

				// Get a handle and connect
				const handle = client.counter.get(["test-get"]);
				const connection = handle.connect();

				// Verify connection by performing an action
				const count = await connection.increment(5);
				expect(count).toBe(5);

				// Clean up
				await connection.dispose();
			});

			test("should connect using .getForId().connect()", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				// Create a actor first to get its ID
				const handle = client.counter.getOrCreate(["test-get-for-id"]);
				await handle.increment(3);
				const actorId = await handle.resolve();

				// Get a new handle using the actor ID and connect
				const idHandle = client.counter.getForId(actorId);
				const connection = idHandle.connect();

				// Verify connection works and state is preserved
				const count = await connection.getCount();
				expect(count).toBe(3);

				// Clean up
				await connection.dispose();
			});

			test("should connect using .getOrCreate().connect()", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				// Get or create actor and connect
				const handle = client.counter.getOrCreate([
					"test-get-or-create",
				]);
				const connection = handle.connect();

				// Verify connection works
				const count = await connection.increment(7);
				expect(count).toBe(7);

				// Clean up
				await connection.dispose();
			});

			test("should connect using (await create()).connect()", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				// Create actor and connect
				const handle = await client.counter.create(["test-create"]);
				const connection = handle.connect();

				// Verify connection works
				const count = await connection.increment(9);
				expect(count).toBe(9);

				// Clean up
				await connection.dispose();
			});
		});

		describe("Event Communication", () => {
			test("should mix RPC calls and WebSocket events", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				// Create actor
				const handle = client.counter.getOrCreate([
					"test-mixed-rpc-ws",
				]);
				const connection = handle.connect();

				// Set up event listener
				const receivedEvents: number[] = [];
				connection.on("newCount", (count: number) => {
					receivedEvents.push(count);
				});

				// TODO: There is a race condition with opening subscription and sending events on SSE, so we need to wait for a successful round trip on the event
				await vi.waitFor(async () => {
					// Send one RPC call over the connection to ensure it's open
					await connection.setCount(1);
					expect(receivedEvents).includes(1);
				});

				// Now use stateless RPC calls through the handle (no connection)
				// These should still trigger events that the connection receives
				await handle.setCount(2);
				await handle.setCount(3);

				// Wait for all events to be received
				await vi.waitFor(() => {
					expect(receivedEvents).includes(2);
					expect(receivedEvents).includes(3);
				});

				// Clean up
				await connection.dispose();
			});

			test("should receive events via broadcast", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				// Create actor and connect
				const handle = client.counter.getOrCreate(["test-broadcast"]);
				const connection = handle.connect();

				// Set up event listener
				const receivedEvents: number[] = [];
				connection.on("newCount", (count: number) => {
					receivedEvents.push(count);
				});

				// HACK: Race condition between subscribing & sending events in SSE
				// Verify events were received
				await vi.waitFor(
					async () => {
						await connection.setCount(5);
						await connection.setCount(8);
						expect(receivedEvents).toContain(5);
						expect(receivedEvents).toContain(8);
					},
					{ timeout: 10_000 },
				);

				// Clean up
				await connection.dispose();
			});

			test("should handle one-time events with once()", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				// Create actor and connect
				const handle = client.counter.getOrCreate(["test-once"]);
				const connection = handle.connect();

				// Set up one-time event listener
				const receivedEvents: number[] = [];
				connection.once("newCount", (count: number) => {
					receivedEvents.push(count);
				});

				// Trigger multiple events, but should only receive the first one
				await connection.increment(5);
				await connection.increment(3);

				// Verify only the first event was received
				await vi.waitFor(() => {
					expect(receivedEvents).toEqual([5]);
					expect(receivedEvents).not.toContain(8);
				});

				// Clean up
				await connection.dispose();
			});

			test("should unsubscribe from events", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				// Create actor and connect
				const handle = client.counter.getOrCreate(["test-unsubscribe"]);
				const connection = handle.connect();

				// Set up event listener with unsubscribe
				const receivedEvents: number[] = [];
				const unsubscribe = connection.on(
					"newCount",
					(count: number) => {
						receivedEvents.push(count);
					},
				);

				// TODO: SSE has race condition with subscriptions & publishing messages
				// Trigger first event
				await vi.waitFor(async () => {
					await connection.setCount(5);
					expect(receivedEvents).toEqual([5]);
				});

				// Unsubscribe
				unsubscribe();

				// Trigger second event, should not be received
				await connection.setCount(8);

				// Verify only the first event was received
				expect(receivedEvents).not.toContain(8);

				// Clean up
				await connection.dispose();
			});
		});

		describe("Connection Parameters", () => {
			test("should pass connection parameters", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				// Create two connections with different params
				const handle1 = client.counterWithParams.getOrCreate(
					["test-params"],
					{
						params: { name: "user1" },
					},
				);
				const handle2 = client.counterWithParams.getOrCreate(
					["test-params"],
					{
						params: { name: "user2" },
					},
				);

				const conn1 = handle1.connect();
				const conn2 = handle2.connect();

				// HACK: Call an action to wait for the connections to be established
				await conn1.getInitializers();
				await conn2.getInitializers();

				// Get initializers to verify connection params were used
				const initializers = await conn1.getInitializers();

				// Verify both connection names were recorded
				expect(initializers).toContain("user1");
				expect(initializers).toContain("user2");

				// Clean up
				await conn1.dispose();
				await conn2.dispose();
			});
		});

		describe("Lifecycle Hooks", () => {
			test("should trigger lifecycle hooks", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				// Create and connect
				const connHandle = client.counterWithLifecycle.getOrCreate(
					["test-lifecycle"],
					{
						params: { trackLifecycle: true },
					},
				);
				const connection = connHandle.connect();

				// Verify lifecycle events were triggered
				const events = await connection.getEvents();
				expect(events).toEqual([
					"onWake",
					"onBeforeConnect",
					"onConnect",
				]);

				// Disconnect should trigger onDisconnect
				await connection.dispose();

				await vi.waitFor(
					async () => {
						// Reconnect to check if onDisconnect was called
						const handle = client.counterWithLifecycle.getOrCreate([
							"test-lifecycle",
						]);
						const finalEvents = await handle.getEvents();
						expect(finalEvents).toBeOneOf([
							// Still active
							[
								"onWake",
								"onBeforeConnect",
								"onConnect",
								"onDisconnect",
							],
							// Went to sleep and woke back up
							[
								"onWake",
								"onBeforeConnect",
								"onConnect",
								"onDisconnect",
								"onWake",
							],
						]);
					},
					// NOTE: High timeout required for Cloudflare Workers
					{
						timeout: 10_000,
						interval: 100,
					},
				);
			});
		});
	});
}
