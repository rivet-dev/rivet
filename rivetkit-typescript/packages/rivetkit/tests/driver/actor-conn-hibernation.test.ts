// @ts-nocheck
import { describeDriverMatrix } from "./shared-matrix";
import { describe, expect, test, vi } from "vitest";
import { HIBERNATION_SLEEP_TIMEOUT } from "../../fixtures/driver-test-suite/hibernation";
import { setupDriverTest, waitFor } from "./shared-utils";

describeDriverMatrix("Actor Conn Hibernation", (driverTestConfig) => {
	describe.skipIf(driverTestConfig.skip?.hibernation)(
		"Connection Hibernation",
		() => {
			test("basic conn hibernation", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				// Create actor with connection
				const hibernatingActor = client.hibernationActor
					.getOrCreate()
					.connect();

				// Initial RPC call
				const ping1 = await hibernatingActor.ping();
				expect(ping1).toBe("pong");

				// Trigger sleep
				await hibernatingActor.triggerSleep();

				// Wait for actor to sleep (give it time to hibernate)
				await waitFor(
					driverTestConfig,
					HIBERNATION_SLEEP_TIMEOUT + 100,
				);

				// Call RPC again - this should wake the actor and work
				const ping2 = await hibernatingActor.ping();
				expect(ping2).toBe("pong");

				// Clean up
				await hibernatingActor.dispose();
			});

			test("conn state persists through hibernation", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				// Create actor with connection
				const hibernatingActor = client.hibernationActor
					.getOrCreate()
					.connect();

				// Increment connection count
				const count1 = await hibernatingActor.connIncrement();
				expect(count1).toBe(1);

				const count2 = await hibernatingActor.connIncrement();
				expect(count2).toBe(2);

				// Get initial lifecycle counts
				const initialLifecycle =
					await hibernatingActor.getConnLifecycleCounts();
				expect(initialLifecycle.connectCount).toBe(1);
				expect(initialLifecycle.disconnectCount).toBe(0);

				// Get initial actor counts
				const initialActorCounts =
					await hibernatingActor.getActorCounts();
				expect(initialActorCounts.wakeCount).toBe(1);
				expect(initialActorCounts.sleepCount).toBe(0);

				// Trigger sleep
				await hibernatingActor.triggerSleep();

				// Wait for actor to sleep
				await waitFor(
					driverTestConfig,
					HIBERNATION_SLEEP_TIMEOUT + 100,
				);

				// Check that connection state persisted
				const count3 = await hibernatingActor.getConnCount();
				expect(count3).toBe(2);

				// Verify lifecycle hooks:
				// - onDisconnect and onConnect should NOT be called during sleep/wake
				// - onSleep and onWake should be called
				const finalLifecycle =
					await hibernatingActor.getConnLifecycleCounts();
				expect(finalLifecycle.connectCount).toBe(1); // No additional connects
				expect(finalLifecycle.disconnectCount).toBe(0); // No disconnects

				const finalActorCounts =
					await hibernatingActor.getActorCounts();
				expect(finalActorCounts.wakeCount).toBe(2); // Woke up once more
				expect(finalActorCounts.sleepCount).toBe(1); // Slept once

				// Clean up
				await hibernatingActor.dispose();
			});

			test("onOpen is not emitted again after hibernation wake", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				const hibernatingActor = client.hibernationActor
					.getOrCreate(["onopen-once"])
					.connect();

				let openCount = 0;
				hibernatingActor.onOpen(() => {
					openCount += 1;
				});

				await vi.waitFor(() => {
					expect(hibernatingActor.isConnected).toBe(true);
					expect(openCount).toBe(1);
				});

				for (let i = 0; i < 2; i++) {
					await hibernatingActor.triggerSleep();
					await waitFor(
						driverTestConfig,
						HIBERNATION_SLEEP_TIMEOUT + 100,
					);

					const ping = await hibernatingActor.ping();
					expect(ping).toBe("pong");

					const actorCounts = await hibernatingActor.getActorCounts();
					expect(actorCounts.sleepCount).toBe(i + 1);
					expect(actorCounts.wakeCount).toBe(i + 2);
					expect(openCount).toBe(1);
				}

				await hibernatingActor.dispose();
			});

			test("closing connection during hibernation", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				// Create actor with first connection
				const conn1 = client.hibernationActor.getOrCreate().connect();

				// Initial RPC call
				await conn1.ping();

				// Get connection ID
				const connectionIds = await conn1.getConnectionIds();
				expect(connectionIds.length).toBe(1);
				const conn1Id = connectionIds[0];

				// Trigger sleep
				await conn1.triggerSleep();

				// Wait for actor to hibernate
				await waitFor(
					driverTestConfig,
					HIBERNATION_SLEEP_TIMEOUT + 100,
				);

				// Disconnect first connection while actor is sleeping
				await conn1.dispose();

				// Wait a bit for disconnection to be processed
				await waitFor(driverTestConfig, 250);

				// Create second connection to verify first connection disconnected
				const conn2 = client.hibernationActor.getOrCreate().connect();

				// Wait for connection to be established
				await vi.waitFor(
					async () => {
						const newConnectionIds = await conn2.getConnectionIds();
						expect(newConnectionIds.length).toBe(1);
						expect(newConnectionIds[0]).not.toBe(conn1Id);
					},
					{
						timeout: 5000,
						interval: 100,
					},
				);

				// Verify onDisconnect was called for the first connection
				const lifecycle = await conn2.getConnLifecycleCounts();
				expect(lifecycle.disconnectCount).toBe(0); // Only for conn2, not conn1

				// Clean up
				await conn2.dispose();
			});

			test("messages sent on a hibernating connection during onSleep resolve after wake", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				for (const delayMs of [0, 100, 400]) {
					const connection = client.hibernationSleepWindowActor
						.getOrCreate([`sleep-window-${delayMs}`])
						.connect();

					await vi.waitFor(async () => {
						expect(connection.isConnected).toBe(true);
					});

					const sleepingPromise = new Promise<void>((resolve) => {
						connection.once("sleeping", () => {
							resolve();
						});
					});

					await connection.triggerSleep();
					await sleepingPromise;

					if (delayMs > 0) {
						await waitFor(driverTestConfig, delayMs);
					}

					const duringSleepPromise = connection.getActorCounts();
					duringSleepPromise.catch(() => {});

					const result = await Promise.race([
						duringSleepPromise
							.then((counts) => ({
								tag: "resolved" as const,
								counts,
							}))
							.catch((error) => ({
								tag: "rejected" as const,
								error:
									error instanceof Error
										? error.message
										: String(error),
							})),
						(async () => {
							await waitFor(driverTestConfig, 3000);
							return { tag: "timed_out" as const };
						})(),
					]);

					expect(result.tag).toBe("resolved");
					if (result.tag === "resolved") {
						expect(result.counts.sleepCount).toBe(1);
						expect(result.counts.wakeCount).toBe(2);
					}

					await connection.dispose();
				}
			});
		},
	);
});
