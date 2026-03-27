import { describe, expect, test } from "vitest";
import { SLEEP_TIMEOUT } from "../../../fixtures/driver-test-suite/sleep";
import type { DriverTestConfig } from "../mod";
import { setupDriverTest, waitFor } from "../utils";

// TODO: These tests are broken with fake timers because `_sleep` requires
// background async promises that have a race condition with calling
// `getCounts`
//
// To fix this, we need to imeplment some event system to be able to check for
// when an actor has slept. OR we can expose an HTTP endpoint on the manager
// for `.test` that checks if na actor is sleeping that we can poll.
export function runActorSleepTests(driverTestConfig: DriverTestConfig) {
	describe.skipIf(driverTestConfig.skip?.sleep)("Actor Sleep Tests", () => {
		test("actor sleep persists state", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);

			// Create actor
			const sleepActor = client.sleep.getOrCreate();

			// Verify initial sleep count
			{
				const { startCount, sleepCount } = await sleepActor.getCounts();
				expect(sleepCount).toBe(0);
				expect(startCount).toBe(1);
			}

			// Trigger sleep
			await sleepActor.triggerSleep();

			// HACK: Wait for sleep to finish in background
			await waitFor(driverTestConfig, 250);

			// Get sleep count after restore
			{
				const { startCount, sleepCount } = await sleepActor.getCounts();
				expect(sleepCount).toBe(1);
				expect(startCount).toBe(2);
			}
		});

		test("actor sleep persists state with connect", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);

			// Create actor with persistent connection
			const sleepActor = client.sleep.getOrCreate().connect();

			// Verify initial sleep count
			{
				const { startCount, sleepCount } = await sleepActor.getCounts();
				expect(sleepCount).toBe(0);
				expect(startCount).toBe(1);
			}

			// Trigger sleep
			await sleepActor.triggerSleep();

			// Disconnect to allow reconnection
			await sleepActor.dispose();

			// HACK: Wait for sleep to finish in background
			await waitFor(driverTestConfig, SLEEP_TIMEOUT + 250);

			// Reconnect to get sleep count after restore
			const sleepActor2 = client.sleep.getOrCreate();
			{
				const { startCount, sleepCount } =
					await sleepActor2.getCounts();
				expect(sleepCount).toBe(1);
				expect(startCount).toBe(2);
			}
		});

		test("actor automatically sleeps after timeout", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);

			// Create actor
			const sleepActor = client.sleep.getOrCreate();

			// Verify initial sleep count
			{
				const { startCount, sleepCount } = await sleepActor.getCounts();
				expect(sleepCount).toBe(0);
				expect(startCount).toBe(1);
			}

			// Wait for sleep
			await waitFor(driverTestConfig, SLEEP_TIMEOUT + 250);

			// Get sleep count after restore
			{
				const { startCount, sleepCount } = await sleepActor.getCounts();
				expect(sleepCount).toBe(1);
				expect(startCount).toBe(2);
			}
		});

		test("actor automatically sleeps after timeout with connect", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);

			// Create actor with persistent connection
			const sleepActor = client.sleep.getOrCreate().connect();

			// Verify initial sleep count
			{
				const { startCount, sleepCount } = await sleepActor.getCounts();
				expect(sleepCount).toBe(0);
				expect(startCount).toBe(1);
			}

			// Disconnect to allow actor to sleep
			await sleepActor.dispose();

			// Wait for sleep
			await waitFor(driverTestConfig, SLEEP_TIMEOUT + 250);

			// Reconnect to get sleep count after restore
			const sleepActor2 = client.sleep.getOrCreate();
			{
				const { startCount, sleepCount } =
					await sleepActor2.getCounts();
				expect(sleepCount).toBe(1);
				expect(startCount).toBe(2);
			}
		});

		test("waitUntil can broadcast before sleep disconnect", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);

			const sleepActor = client.sleepWithWaitUntilMessage
				.getOrCreate()
				.connect();
			const receivedMessages: Array<{
				sleepCount: number;
				startCount: number;
			}> = [];

			sleepActor.once("sleeping", (message) => {
				receivedMessages.push(message);
			});

			await sleepActor.triggerSleep();
			await waitFor(driverTestConfig, 250);

			expect(receivedMessages).toHaveLength(1);
			expect(receivedMessages[0]?.startCount).toBe(1);

			await sleepActor.dispose();

			await waitFor(driverTestConfig, 250);

			const sleepActor2 = client.sleepWithWaitUntilMessage.getOrCreate();
			{
				const { startCount, sleepCount, waitUntilMessageCount } =
					await sleepActor2.getCounts();
				expect(waitUntilMessageCount).toBe(1);
				expect(sleepCount).toBe(1);
				expect(startCount).toBe(2);
			}
		});

		test("rpc calls keep actor awake", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);

			// Create actor
			const sleepActor = client.sleep.getOrCreate();

			// Verify initial state
			{
				const { startCount, sleepCount } = await sleepActor.getCounts();
				expect(sleepCount).toBe(0);
				expect(startCount).toBe(1);
			}

			// Wait almost until sleep timeout, then make RPC call
			await waitFor(driverTestConfig, SLEEP_TIMEOUT - 250);

			// RPC call should reset the sleep timer
			{
				const { startCount, sleepCount } = await sleepActor.getCounts();
				expect(sleepCount).toBe(0); // Haven't slept yet
				expect(startCount).toBe(1); // Still the same instance
			}

			// Wait another partial timeout period - actor should still be awake
			await waitFor(driverTestConfig, SLEEP_TIMEOUT - 250);

			// Actor should still be awake
			{
				const { startCount, sleepCount } = await sleepActor.getCounts();
				expect(sleepCount).toBe(0); // Still haven't slept
				expect(startCount).toBe(1); // Still the same instance
			}

			// Now wait for full timeout without any RPC calls
			await waitFor(driverTestConfig, SLEEP_TIMEOUT + 250);

			// Actor should have slept and restarted
			{
				const { startCount, sleepCount } = await sleepActor.getCounts();
				expect(sleepCount).toBe(1); // Slept once
				expect(startCount).toBe(2); // New instance after sleep
			}
		});

		test("alarms keep actor awake", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);

			// Create actor
			const sleepActor = client.sleep.getOrCreate();

			// Verify initial state
			{
				const { startCount, sleepCount } = await sleepActor.getCounts();
				expect(sleepCount).toBe(0);
				expect(startCount).toBe(1);
			}

			// Set an alarm to keep the actor awake
			await sleepActor.setAlarm(SLEEP_TIMEOUT - 250);

			// Wait until after SLEEPT_IMEOUT to validate the actor did not sleep
			await waitFor(driverTestConfig, SLEEP_TIMEOUT + 250);

			// Actor should not have slept
			{
				const { startCount, sleepCount } = await sleepActor.getCounts();
				expect(sleepCount).toBe(0);
				expect(startCount).toBe(1);
			}
		});

		test("alarms wake actors", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);

			// Create actor
			const sleepActor = client.sleep.getOrCreate();

			// Verify initial state
			{
				const { startCount, sleepCount } = await sleepActor.getCounts();
				expect(sleepCount).toBe(0);
				expect(startCount).toBe(1);
			}

			// Set an alarm to keep the actor awake
			await sleepActor.setAlarm(SLEEP_TIMEOUT + 250);

			// Wait until after SLEEPT_IMEOUT to validate the actor did not sleep
			await waitFor(driverTestConfig, SLEEP_TIMEOUT + 200);

			// Actor should not have slept
			{
				const { startCount, sleepCount } = await sleepActor.getCounts();
				expect(sleepCount).toBe(1);
				expect(startCount).toBe(2);
			}
		});

		test("long running rpcs keep actor awake", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);

			// Create actor
			const sleepActor = client.sleepWithLongRpc.getOrCreate().connect();

			// Verify initial state
			{
				const { startCount, sleepCount } = await sleepActor.getCounts();
				expect(sleepCount).toBe(0);
				expect(startCount).toBe(1);
			}

			// Start a long-running RPC that takes longer than the sleep timeout
			const waitPromise = new Promise((resolve) =>
				sleepActor.once("waiting", resolve),
			);
			const longRunningPromise = sleepActor.longRunningRpc();
			await waitPromise;
			await waitFor(driverTestConfig, SLEEP_TIMEOUT + 250);
			await sleepActor.finishLongRunningRpc();
			await longRunningPromise;

			// Actor should still be the same instance (didn't sleep during RPC)
			{
				const { startCount, sleepCount } = await sleepActor.getCounts();
				expect(sleepCount).toBe(0); // Hasn't slept
				expect(startCount).toBe(1); // Same instance
			}
			await sleepActor.dispose();

			// Now wait for the sleep timeout
			await waitFor(driverTestConfig, SLEEP_TIMEOUT + 250);

			// Actor should have slept after the timeout
			const sleepActor2 = client.sleepWithLongRpc.getOrCreate();
			{
				const { startCount, sleepCount } =
					await sleepActor2.getCounts();
				expect(sleepCount).toBe(1); // Slept once
				expect(startCount).toBe(2); // New instance after sleep
			}
		});

		test("active raw websockets keep actor awake", async (c) => {
			const { client, endpoint: baseUrl } = await setupDriverTest(
				c,
				driverTestConfig,
			);

			// Create actor
			const sleepActor = client.sleepWithRawWebSocket.getOrCreate();

			// Verify initial state
			{
				const { startCount, sleepCount } = await sleepActor.getCounts();
				expect(sleepCount).toBe(0);
				expect(startCount).toBe(1);
			}

			// Connect WebSocket
			const ws = await sleepActor.webSocket();

			await new Promise<void>((resolve, reject) => {
				ws.onopen = () => resolve();
				ws.onerror = reject;
			});

			// Wait for connection message
			await new Promise<void>((resolve) => {
				ws.onmessage = (event: { data: string }) => {
					const data = JSON.parse(event.data);
					if (data.type === "connected") {
						resolve();
					}
				};
			});

			// Wait longer than sleep timeout while keeping WebSocket connected
			await waitFor(driverTestConfig, SLEEP_TIMEOUT + 250);

			// Send a message to check if actor is still alive
			ws.send(JSON.stringify({ type: "getCounts" }));

			const counts = await new Promise<any>((resolve) => {
				ws.onmessage = (event: { data: string }) => {
					const data = JSON.parse(event.data);
					if (data.type === "counts") {
						resolve(data);
					}
				};
			});

			// Actor should still be the same instance (didn't sleep while WebSocket connected)
			expect(counts.sleepCount).toBe(0);
			expect(counts.startCount).toBe(1);

			// Close WebSocket
			ws.close();

			// Wait for sleep timeout after WebSocket closed
			await waitFor(driverTestConfig, SLEEP_TIMEOUT + 250);

			// Actor should have slept after WebSocket closed
			{
				const { startCount, sleepCount } = await sleepActor.getCounts();
				expect(sleepCount).toBe(1); // Slept once
				expect(startCount).toBe(2); // New instance after sleep
			}
		});

		test("active raw fetch requests keep actor awake", async (c) => {
			const { client, endpoint: baseUrl } = await setupDriverTest(
				c,
				driverTestConfig,
			);

			// Create actor
			const sleepActor = client.sleepWithRawHttp.getOrCreate();

			// Verify initial state
			{
				const { startCount, sleepCount } = await sleepActor.getCounts();
				expect(sleepCount).toBe(0);
				expect(startCount).toBe(1);
			}

			// Start a long-running fetch request
			const fetchDuration = SLEEP_TIMEOUT + 250;
			const fetchPromise = sleepActor.fetch(
				`long-request?duration=${fetchDuration}`,
			);

			// Wait for the fetch to complete
			const response = await fetchPromise;
			const result = (await response.json()) as { completed: boolean };
			expect(result.completed).toBe(true);
			{
				const { startCount, sleepCount, requestCount } =
					await sleepActor.getCounts();
				expect(sleepCount).toBe(0);
				expect(startCount).toBe(1);
				expect(requestCount).toBe(1);
			}

			// Wait for sleep timeout
			await waitFor(driverTestConfig, SLEEP_TIMEOUT + 250);

			// Actor should have slept after timeout
			{
				const { startCount, sleepCount } = await sleepActor.getCounts();
				expect(sleepCount).toBe(1); // Slept once
				expect(startCount).toBe(2); // New instance after sleep
			}
		});

		test("noSleep option disables sleeping", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);

			// Create actor with noSleep option
			const sleepActor = client.sleepWithNoSleepOption.getOrCreate();

			// Verify initial state
			{
				const { startCount, sleepCount } = await sleepActor.getCounts();
				expect(sleepCount).toBe(0);
				expect(startCount).toBe(1);
			}

			// Wait longer than sleep timeout
			await waitFor(driverTestConfig, SLEEP_TIMEOUT + 250);

			// Actor should NOT have slept due to noSleep option
			{
				const { startCount, sleepCount } = await sleepActor.getCounts();
				expect(sleepCount).toBe(0); // Never slept
				expect(startCount).toBe(1); // Still the same instance
			}

			// Wait even longer to be sure
			await waitFor(driverTestConfig, SLEEP_TIMEOUT + 250);

			// Actor should still not have slept
			{
				const { startCount, sleepCount } = await sleepActor.getCounts();
				expect(sleepCount).toBe(0); // Never slept
				expect(startCount).toBe(1); // Still the same instance
			}
		});

		test("preventSleep blocks auto sleep until cleared", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);

			const sleepActor = client.sleepWithPreventSleep.getOrCreate();

			{
				const status = await sleepActor.getStatus();
				expect(status.sleepCount).toBe(0);
				expect(status.startCount).toBe(1);
				expect(status.preventSleep).toBe(false);
				expect(status.preventSleepOnWake).toBe(false);
			}

			expect(await sleepActor.setPreventSleep(true)).toBe(true);

			await waitFor(driverTestConfig, SLEEP_TIMEOUT + 250);

			{
				const status = await sleepActor.getStatus();
				expect(status.sleepCount).toBe(0);
				expect(status.startCount).toBe(1);
				expect(status.preventSleep).toBe(true);
			}

			expect(await sleepActor.setPreventSleep(false)).toBe(false);

			await waitFor(driverTestConfig, SLEEP_TIMEOUT + 250);

			{
				const status = await sleepActor.getStatus();
				expect(status.sleepCount).toBe(1);
				expect(status.startCount).toBe(2);
				expect(status.preventSleep).toBe(false);
			}
		});

		test("preventSleep can be restored during onWake", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);

			const sleepActor = client.sleepWithPreventSleep.getOrCreate();

			expect(await sleepActor.setPreventSleepOnWake(true)).toBe(true);

			await sleepActor.triggerSleep();
			await waitFor(driverTestConfig, 250);

			{
				const status = await sleepActor.getStatus();
				expect(status.sleepCount).toBe(1);
				expect(status.startCount).toBe(2);
				expect(status.preventSleep).toBe(true);
				expect(status.preventSleepOnWake).toBe(true);
			}

			await waitFor(driverTestConfig, SLEEP_TIMEOUT + 250);

			{
				const status = await sleepActor.getStatus();
				expect(status.sleepCount).toBe(1);
				expect(status.startCount).toBe(2);
				expect(status.preventSleep).toBe(true);
			}

			expect(await sleepActor.setPreventSleepOnWake(false)).toBe(false);
			expect(await sleepActor.setPreventSleep(false)).toBe(false);

			await waitFor(driverTestConfig, SLEEP_TIMEOUT + 250);

			{
				const status = await sleepActor.getStatus();
				expect(status.sleepCount).toBe(2);
				expect(status.startCount).toBe(3);
				expect(status.preventSleep).toBe(false);
				expect(status.preventSleepOnWake).toBe(false);
			}
		});

		test("onSleep sends message to raw websocket", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);

			const sleepActor = client.sleepRawWsSendOnSleep.getOrCreate();

			// Connect WebSocket
			const ws = await sleepActor.webSocket();

			await new Promise<void>((resolve, reject) => {
				ws.onopen = () => resolve();
				ws.onerror = reject;
			});

			// Wait for connected message
			await new Promise<void>((resolve) => {
				ws.onmessage = (event: { data: string }) => {
					const data = JSON.parse(event.data);
					if (data.type === "connected") {
						resolve();
					}
				};
			});

			// Listen for the sleeping message or close event
			const result = await new Promise<{
				message: any | null;
				closed: boolean;
			}>((resolve) => {
				ws.onmessage = (event: { data: string }) => {
					const data = JSON.parse(event.data);
					if (data.type === "sleeping") {
						resolve({ message: data, closed: false });
					}
				};
				ws.onclose = () => {
					resolve({ message: null, closed: true });
				};

				// Trigger sleep after handlers are set up
				sleepActor.triggerSleep();
			});

			// The message should have been received
			expect(result.message).toBeDefined();
			expect(result.message?.type).toBe("sleeping");
			expect(result.message?.sleepCount).toBe(1);

			// Close the WebSocket from client side
			ws.close();

			// Wait for sleep to fully complete
			await waitFor(driverTestConfig, 500);

			// Verify sleep happened
			{
				const { startCount, sleepCount } =
					await sleepActor.getCounts();
				expect(sleepCount).toBe(1);
				expect(startCount).toBe(2);
			}
		});

		test("onSleep sends delayed message to raw websocket", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);

			const sleepActor =
				client.sleepRawWsDelayedSendOnSleep.getOrCreate();

			// Connect WebSocket
			const ws = await sleepActor.webSocket();

			await new Promise<void>((resolve, reject) => {
				ws.onopen = () => resolve();
				ws.onerror = reject;
			});

			// Wait for connected message
			await new Promise<void>((resolve) => {
				ws.onmessage = (event: { data: string }) => {
					const data = JSON.parse(event.data);
					if (data.type === "connected") {
						resolve();
					}
				};
			});

			// Listen for the sleeping message or close event
			const result = await new Promise<{
				message: any | null;
				closed: boolean;
			}>((resolve) => {
				ws.onmessage = (event: { data: string }) => {
					const data = JSON.parse(event.data);
					if (data.type === "sleeping") {
						resolve({ message: data, closed: false });
					}
				};
				ws.onclose = () => {
					resolve({ message: null, closed: true });
				};

				// Trigger sleep after handlers are set up
				sleepActor.triggerSleep();
			});

			// The message should have been received after the delay
			expect(result.message).toBeDefined();
			expect(result.message?.type).toBe("sleeping");
			expect(result.message?.sleepCount).toBe(1);

			// Close the WebSocket from client side
			ws.close();

			// Wait for sleep to fully complete
			await waitFor(driverTestConfig, 500);

			// Verify sleep happened
			{
				const { startCount, sleepCount } =
					await sleepActor.getCounts();
				expect(sleepCount).toBe(1);
				expect(startCount).toBe(2);
			}
		});
	});
}
