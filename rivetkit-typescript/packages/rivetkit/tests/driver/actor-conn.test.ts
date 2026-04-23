// @ts-nocheck

import { describe, expect, test, vi } from "vitest";
import { describeDriverMatrix } from "./shared-matrix";
import { setupDriverTest } from "./shared-utils";

const CONNECTION_BOOTSTRAP_TIMEOUT_MS = 20_000;

async function waitForConnectionBootstrap<T>(
	action: () => Promise<T>,
): Promise<T> {
	let result: T | undefined;
	// Poll because actor-connect bootstrap can race registry setup and only becomes callable once the socket is ready.
	await vi.waitFor(
		async () => {
			result = await action();
		},
		{ timeout: CONNECTION_BOOTSTRAP_TIMEOUT_MS, interval: 100 },
	);
	if (result === undefined) {
		throw new Error("connection bootstrap never produced a result");
	}
	return result;
}

function collectConnectionEvents<T>(
	connection: {
		on(eventName: string, callback: (value: T) => void): () => void;
	},
	eventName: string,
	count: number,
): Promise<T[]> {
	return new Promise((resolve) => {
		const values: T[] = [];
		const unsubscribe = connection.on(eventName, (value: T) => {
			values.push(value);
			if (values.length === count) {
				unsubscribe();
				resolve(values);
			}
		});
	});
}

describeDriverMatrix("Actor Conn", (driverTestConfig) => {
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

				const bootstrapEventPromise = collectConnectionEvents<number>(
					connection,
					"newCount",
					1,
				);
				await connection.setCount(1);
				expect(await bootstrapEventPromise).toEqual([1]);

				// Now use stateless RPC calls through the handle (no connection)
				// These should still trigger events that the connection receives
				const forwardedEventsPromise = collectConnectionEvents<number>(
					connection,
					"newCount",
					2,
				);
				await handle.setCount(2);
				await handle.setCount(3);
				expect(await forwardedEventsPromise).toEqual([2, 3]);
				expect(receivedEvents).toEqual([1, 2, 3]);

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

				const broadcastEventsPromise = collectConnectionEvents<number>(
					connection,
					"newCount",
					2,
				);
				await connection.setCount(5);
				await connection.setCount(8);
				expect(await broadcastEventsPromise).toEqual([5, 8]);
				expect(receivedEvents).toEqual([5, 8]);

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
				const firstEventPromise = new Promise<number>((resolve) => {
					connection.once("newCount", (count: number) => {
						receivedEvents.push(count);
						resolve(count);
					});
				});

				// Trigger multiple events, but should only receive the first one
				await connection.increment(5);
				await connection.increment(3);

				expect(await firstEventPromise).toBe(5);
				expect(receivedEvents).toEqual([5]);
				expect(receivedEvents).not.toContain(8);

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
				let resolveFirstEvent: ((value: number) => void) | undefined;
				const unsubscribe = connection.on(
					"newCount",
					(count: number) => {
						receivedEvents.push(count);
						resolveFirstEvent?.(count);
						resolveFirstEvent = undefined;
					},
				);
				const firstEventPromise = new Promise<number>((resolve) => {
					resolveFirstEvent = resolve;
				});

				await connection.setCount(5);
				expect(await firstEventPromise).toBe(5);
				expect(receivedEvents).toEqual([5]);

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
				await waitForConnectionBootstrap(() => conn1.getInitializers());

				const conn2 = handle2.connect();
				await waitForConnectionBootstrap(() => conn2.getInitializers());

				// Get initializers to verify connection params were used
				const initializers = await waitForConnectionBootstrap(() =>
					conn1.getInitializers(),
				);

				// Verify both connection names were recorded
				expect(initializers).toContain("user1");
				expect(initializers).toContain("user2");

				// Clean up
				await conn1.dispose();
				await conn2.dispose();
			});

			test("should call getParams for each connection", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				let connectionCount = 0;
				const handle = client.counterWithParams.getOrCreate(
					["test-get-params"],
					{
						getParams: async () => ({
							name: `user${++connectionCount}`,
						}),
					},
				);

				const conn1 = handle.connect();
				await waitForConnectionBootstrap(() => conn1.getInitializers());
				await conn1.dispose();

				const conn2 = handle.connect();
				const initializers = await waitForConnectionBootstrap(() =>
					conn2.getInitializers(),
				);

				expect(initializers).toEqual(["user1", "user2"]);
				expect(connectionCount).toBe(2);

				await conn2.dispose();
			});

			test("should surface getParams errors and retry connection setup", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				let attempts = 0;
				const handle = client.counterWithParams.getOrCreate(
					["test-get-params-retry"],
					{
						getParams: async () => {
							attempts++;
							if (attempts === 1) {
								throw new Error("token unavailable");
							}

							return { name: "user1" };
						},
					},
				);

				const conn = handle.connect();
				const receivedErrors: Array<{ group: string; code: string }> =
					[];
				conn.onError((error) => {
					receivedErrors.push({
						group: error.group,
						code: error.code,
					});
				});

				await expect(conn.getInitializers()).rejects.toMatchObject({
					group: "client",
					code: "get_params_failed",
				});

				// Poll until the retrying connection setup succeeds and exposes the settled initializer state.
				await vi.waitFor(
					async () => {
						expect(await conn.getInitializers()).toEqual(["user1"]);
					},
					{ timeout: 30_000 },
				);

				expect(receivedErrors).toEqual([
					{ group: "client", code: "get_params_failed" },
				]);
				expect(attempts).toBeGreaterThanOrEqual(2);

				await conn.dispose();
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

				// Poll because onDisconnect runs asynchronously after the transport closes.
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

		describe("Connection State", () => {
			test("isConnected should be false before connection opens", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				// Create actor and get connection
				const handle = client.counter.getOrCreate([
					"test-isconnected-initial",
				]);
				const connection = handle.connect();

				// isConnected should be false initially (connection not yet established)
				expect(connection.isConnected).toBe(false);

				// Poll until the async connect handshake flips isConnected on the client handle.
				await vi.waitFor(
					() => {
						expect(connection.isConnected).toBe(true);
					},
					{ timeout: 10_000, interval: 25 },
				);

				// Clean up
				await connection.dispose();
			});

			test("onOpen should be called when connection opens", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				// Create actor and get connection
				const handle = client.counter.getOrCreate(["test-onopen"]);
				const connection = handle.connect();

				// Track open events
				let openCount = 0;
				connection.onOpen(() => {
					openCount++;
				});

				// The open callback depends on the async WebSocket init round trip.
				await vi.waitFor(
					() => {
						expect(openCount).toBe(1);
					},
					{ timeout: 10_000, interval: 25 },
				);

				// Verify isConnected is true
				expect(connection.isConnected).toBe(true);

				// Clean up
				await connection.dispose();
			});

			test("onClose should be called when connection closes via dispose", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				// Create actor and get connection
				const handle = client.counter.getOrCreate(["test-onclose"]);
				const connection = handle.connect();

				// Track close events
				let closeCount = 0;
				connection.onClose(() => {
					closeCount++;
				});

				// Connection opening depends on the async WebSocket init round trip.
				await vi.waitFor(
					() => {
						expect(connection.isConnected).toBe(true);
					},
					{ timeout: 10_000, interval: 25 },
				);

				// Dispose connection
				await connection.dispose();

				// Verify onClose was called
				expect(closeCount).toBe(1);

				// Verify isConnected is false
				expect(connection.isConnected).toBe(false);
			});

			test("should be able to unsubscribe from onOpen", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				// Create actor and get connection
				const handle = client.counter.getOrCreate([
					"test-onopen-unsub",
				]);
				const connection = handle.connect();

				// Track open events
				let openCount = 0;
				const unsubscribe = connection.onOpen(() => {
					openCount++;
				});

				// Unsubscribe immediately
				unsubscribe();

				// Connection opening depends on the async WebSocket init round trip.
				await vi.waitFor(
					() => {
						expect(connection.isConnected).toBe(true);
					},
					{ timeout: 10_000, interval: 25 },
				);

				// Open callback should not have been called since we unsubscribed
				expect(openCount).toBe(0);

				// Clean up
				await connection.dispose();
			});

			test("should be able to unsubscribe from onClose", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				// Create actor and get connection
				const handle = client.counter.getOrCreate([
					"test-onclose-unsub",
				]);
				const connection = handle.connect();

				// Track close events
				let closeCount = 0;
				const unsubscribe = connection.onClose(() => {
					closeCount++;
				});

				// Connection opening depends on the async WebSocket init round trip.
				await vi.waitFor(
					() => {
						expect(connection.isConnected).toBe(true);
					},
					{ timeout: 10_000, interval: 25 },
				);

				// Unsubscribe before closing
				unsubscribe();

				// Dispose connection
				await connection.dispose();

				// Close callback should not have been called since we unsubscribed
				expect(closeCount).toBe(0);
			});

			test("multiple onOpen handlers should all be called", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				// Create actor and get connection
				const handle = client.counter.getOrCreate([
					"test-multi-onopen",
				]);
				const connection = handle.connect();

				// Track open events from multiple handlers
				let handler1Called = false;
				let handler2Called = false;

				connection.onOpen(() => {
					handler1Called = true;
				});
				connection.onOpen(() => {
					handler2Called = true;
				});

				// Poll until both async onOpen callbacks fire after the connect handshake completes.
				await vi.waitFor(
					() => {
						expect(handler1Called).toBe(true);
						expect(handler2Called).toBe(true);
					},
					{ timeout: 10_000, interval: 25 },
				);

				// Clean up
				await connection.dispose();
			});

			test("multiple onClose handlers should all be called", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				// Create actor and get connection
				const handle = client.counter.getOrCreate([
					"test-multi-onclose",
				]);
				const connection = handle.connect();

				// Track close events from multiple handlers
				let handler1Called = false;
				let handler2Called = false;

				connection.onClose(() => {
					handler1Called = true;
				});
				connection.onClose(() => {
					handler2Called = true;
				});

				// Poll until the async connect handshake finishes before exercising onClose handlers.
				await vi.waitFor(
					() => {
						expect(connection.isConnected).toBe(true);
					},
					{ timeout: 10_000, interval: 25 },
				);

				// Dispose connection
				await connection.dispose();

				// Verify both handlers were called
				expect(handler1Called).toBe(true);
				expect(handler2Called).toBe(true);
			});
		});

		describe("Large Payloads", () => {
			test("should handle large request within size limit", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				const handle = client.largePayloadConnActor.getOrCreate([
					"test-large-request",
				]);
				const connection = handle.connect();

				// Create a large payload that's under the default 64KB limit
				// Each item is roughly 60 bytes, so 800 items ≈ 48KB
				const items: string[] = [];
				for (let i = 0; i < 800; i++) {
					items.push(
						`Item ${i} with some additional text to increase size`,
					);
				}

				const result = await connection.processLargeRequest({ items });

				expect(result.itemCount).toBe(800);
				expect(result.firstItem).toBe(
					"Item 0 with some additional text to increase size",
				);
				expect(result.lastItem).toBe(
					"Item 799 with some additional text to increase size",
				);

				// Verify connection state was updated
				const lastRequestSize = await connection.getLastRequestSize();
				expect(lastRequestSize).toBe(800);

				// Clean up
				await connection.dispose();
			});

			test("should reject request exceeding maxIncomingMessageSize", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				const handle = client.largePayloadConnActor.getOrCreate([
					"test-large-request-exceed",
				]);
				const connection = handle.connect();

				// Create a payload that exceeds the default 64KB limit
				// Each item is roughly 60 bytes, so 1500 items ≈ 90KB
				const items: string[] = [];
				for (let i = 0; i < 1500; i++) {
					items.push(
						`Item ${i} with some additional text to increase size`,
					);
				}

				await expect(
					connection.processLargeRequest({ items }),
				).rejects.toMatchObject({
					group: "message",
					code: "incoming_too_long",
				});

				// Clean up
				await connection.dispose();
			});

			test("should handle large response", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				const handle = client.largePayloadConnActor.getOrCreate([
					"test-large-response",
				]);
				const connection = handle.connect();

				// Request a large response (800 items ≈ 48KB)
				const result = await connection.getLargeResponse(800);

				expect(result.items).toHaveLength(800);
				expect(result.items[0]).toBe(
					"Item 0 with some additional text to increase size",
				);
				expect(result.items[799]).toBe(
					"Item 799 with some additional text to increase size",
				);

				// Clean up
				await connection.dispose();
			});

			test("should reject response exceeding maxOutgoingMessageSize", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				const handle = client.largePayloadConnActor.getOrCreate([
					"test-large-response-exceed",
				]);
				const connection = handle.connect();

				// Request a response that exceeds the default 1MB limit
				// Each item is roughly 60 bytes, so 20000 items ≈ 1.2MB
				await expect(
					connection.getLargeResponse(20000),
				).rejects.toMatchObject({
					group: "message",
					code: "outgoing_too_long",
				});

				// Clean up
				await connection.dispose();
			});
		});
	});
});
