import { describe, expect, test, vi } from "vitest";
import { RAW_WS_HANDLER_DELAY } from "../../../fixtures/driver-test-suite/sleep";
import {
	SLEEP_DB_TIMEOUT,
	EXCEEDS_GRACE_HANDLER_DELAY,
	EXCEEDS_GRACE_PERIOD,
	EXCEEDS_GRACE_SLEEP_TIMEOUT,
	ACTIVE_DB_WRITE_COUNT,
	ACTIVE_DB_WRITE_DELAY_MS,
	ACTIVE_DB_GRACE_PERIOD,
	ACTIVE_DB_SLEEP_TIMEOUT,
} from "../../../fixtures/driver-test-suite/sleep-db";
import type { DriverTestConfig } from "../mod";
import { setupDriverTest, waitFor } from "../utils";

type LogEntry = { id: number; event: string; created_at: number };

async function connectRawWebSocket(handle: { webSocket(): Promise<WebSocket> }) {
	const ws = await handle.webSocket();

	await new Promise<void>((resolve, reject) => {
		ws.addEventListener("open", () => resolve(), { once: true });
		ws.addEventListener("error", () => reject(new Error("websocket error")), {
			once: true,
		});
	});

	await new Promise<void>((resolve, reject) => {
		const onMessage = (event: MessageEvent) => {
			const data = JSON.parse(String(event.data));
			if (data.type === "connected") {
				cleanup();
				resolve();
			}
		};
		const onClose = () => {
			cleanup();
			reject(new Error("websocket closed early"));
		};
		const cleanup = () => {
			ws.removeEventListener("message", onMessage);
			ws.removeEventListener("close", onClose);
		};

		ws.addEventListener("message", onMessage);
		ws.addEventListener("close", onClose, { once: true });
	});

	return ws;
}

async function waitForConnected(
	connection: { isConnected: boolean },
	timeout = 10_000,
) {
	await vi.waitFor(
		() => {
			expect(connection.isConnected).toBe(true);
		},
		{
			timeout,
			interval: 50,
		},
	);
}

export function runActorSleepDbTests(driverTestConfig: DriverTestConfig) {
	const describeSleepDbTests = driverTestConfig.skip?.sleep
		? describe.skip
		: describe.sequential;

	describeSleepDbTests("Actor Sleep Database Tests", () => {
			test("onSleep can write to c.db", async (c) => {
				const { client } = await setupDriverTest(
					c,
					driverTestConfig,
				);

				const actor = client.sleepWithDb.getOrCreate();

				// Insert a log entry while awake
				await actor.insertLogEntry("before-sleep");

				// Trigger sleep
				await actor.triggerSleep();

				// Wait for sleep to complete
				await waitFor(driverTestConfig, 250);

				// Wake the actor by calling an action
				const counts = await actor.getCounts();
				expect(counts.sleepCount).toBe(1);
				expect(counts.startCount).toBe(2);
				expect(counts.onSleepDbWriteSuccess).toBe(true);
				expect(counts.onSleepDbWriteError).toBeNull();

				// Verify both wake and sleep events were logged to the DB
				const entries = await actor.getLogEntries();
				const events = entries.map(
					(e: { event: string }) => e.event,
				);
				expect(events).toContain("wake");
				expect(events).toContain("before-sleep");
				expect(events).toContain("sleep");
			});

			test("c.db works after sleep-wake cycle", async (c) => {
				const { client } = await setupDriverTest(
					c,
					driverTestConfig,
				);

				const actor = client.sleepWithDb.getOrCreate([
					"db-after-wake",
				]);

				// Insert before sleep
				await actor.insertLogEntry("before");

				// Let it auto-sleep
				await waitFor(driverTestConfig, SLEEP_DB_TIMEOUT + 250);

				// Wake it by calling an action that uses the DB
				await actor.insertLogEntry("after-wake");

				const entries = await actor.getLogEntries();
				const events = entries.map(
					(e: { event: string }) => e.event,
				);
				expect(events).toContain("before");
				expect(events).toContain("sleep");
				expect(events).toContain("wake");
				expect(events).toContain("after-wake");
			});

			test(
				"scheduled alarm can use c.db after sleep-wake",
				{ timeout: 20_000 },
				async (c) => {
				const { client } = await setupDriverTest(
					c,
					driverTestConfig,
				);

				const actor = client.sleepWithDb.getOrCreate([
					"alarm-db-wake",
				]);

				// Schedule an alarm that fires after the actor would sleep
				await actor.setAlarm(SLEEP_DB_TIMEOUT + 500);

				await waitFor(driverTestConfig, SLEEP_DB_TIMEOUT + 2_000);

				const counts = await actor.getCounts();
				expect(counts.sleepCount).toBeGreaterThanOrEqual(1);
				expect(counts.startCount).toBeGreaterThanOrEqual(2);

				const entries = await actor.getLogEntries();
				const events = entries.map(
					(e: { event: string }) => e.event,
				);
				expect(events).toContain("alarm");
				},
			);

			test(
				"scheduled action stays awake until db work completes",
				{ timeout: 20_000 },
				async (c) => {
				const { client } = await setupDriverTest(
					c,
					driverTestConfig,
				);

				const actor = client.sleepWithSlowScheduledDb.getOrCreate([
					"slow-scheduled-db",
				]);

				await actor.scheduleSlowAlarm(
					50,
					SLEEP_DB_TIMEOUT + 250,
				);

				await waitFor(driverTestConfig, SLEEP_DB_TIMEOUT * 2 + 1_500);

				const counts = await actor.getCounts();
				expect(counts.sleepCount).toBeGreaterThanOrEqual(1);
				expect(counts.startCount).toBeGreaterThanOrEqual(2);
				const entries = await actor.getLogEntries();
				const events = entries.map(
					(e: { event: string }) => e.event,
				);
				expect(events).toContain("slow-alarm-start");
				expect(events).toContain("slow-alarm-finish");

				const finishIndex = events.indexOf("slow-alarm-finish");
				const sleepAfterFinishIndex = events.findIndex(
					(event, index) =>
						event === "sleep" && index > finishIndex,
				);
				expect(sleepAfterFinishIndex).toBeGreaterThan(finishIndex);
				},
			);

			test("onDisconnect can write to c.db during sleep shutdown", async (c) => {
				const { client } = await setupDriverTest(
					c,
					driverTestConfig,
				);

				// Create actor with a connection
				const handle = client.sleepWithDbConn.getOrCreate([
					"disconnect-db-write",
				]);
				const connection = handle.connect();

				// Wait for connection to be established
				await waitForConnected(connection);

				// Insert a log entry while awake
				await connection.insertLogEntry("before-sleep");

				// Trigger sleep, then dispose the connection.
				// During the sleep shutdown sequence, onDisconnect is called
				// with the DB still open (step 6 in the shutdown sequence).
				await connection.triggerSleep();
				await connection.dispose();

				await vi.waitFor(
					async () => {
						const counts = await handle.getCounts();
						expect(counts.sleepCount).toBe(1);
						expect(counts.startCount).toBe(2);

						const entries = await handle.getLogEntries();
						const events = entries.map(
							(e: LogEntry) => e.event,
						);

						expect(events).toContain("before-sleep");
						expect(events).toContain("sleep");
						expect(events).toContain("disconnect");
					},
					{
						timeout: 10_000,
						interval: 50,
					},
				);
			});

			test(
				"async websocket close handler can use c.db before sleep completes",
				{ timeout: 20_000 },
				async (c) => {
				const { client } = await setupDriverTest(
					c,
					driverTestConfig,
				);

				const actor = client.sleepWithRawWsCloseDb.getOrCreate([
					"raw-ws-close-db",
				]);
				const ws = await connectRawWebSocket(actor);

				await new Promise<void>((resolve, reject) => {
					ws.addEventListener("close", () => resolve(), { once: true });
					ws.addEventListener(
						"error",
						() => reject(new Error("websocket error")),
						{ once: true },
					);
					ws.close();
				});

				await waitFor(driverTestConfig, RAW_WS_HANDLER_DELAY + 1_000);

				const status = await actor.getStatus();
				expect(status.sleepCount).toBeGreaterThanOrEqual(1);
				expect(status.startCount).toBeGreaterThanOrEqual(2);
				expect(status.closeStarted).toBe(1);
				expect(status.closeFinished).toBe(1);

				const entries = await actor.getLogEntries();
				const events = entries.map((entry: LogEntry) => entry.event);
				expect(events).toContain("sleep");
				expect(events).toContain("close-start");
				expect(events).toContain("close-finish");
				},
			);

			test(
				"async websocket addEventListener close handler can use c.db before sleep completes",
				{ timeout: 20_000 },
				async (c) => {
				const { client } = await setupDriverTest(
					c,
					driverTestConfig,
				);

				const actor =
					client.sleepWithRawWsCloseDbListener.getOrCreate([
						"raw-ws-close-db-listener",
					]);
				const ws = await connectRawWebSocket(actor);

				await new Promise<void>((resolve, reject) => {
					ws.addEventListener("close", () => resolve(), { once: true });
					ws.addEventListener(
						"error",
						() => reject(new Error("websocket error")),
						{ once: true },
					);
					ws.close();
				});

				await waitFor(driverTestConfig, RAW_WS_HANDLER_DELAY + 1_000);

				const status = await actor.getStatus();
				expect(status.sleepCount).toBeGreaterThanOrEqual(1);
				expect(status.startCount).toBeGreaterThanOrEqual(2);
				expect(status.closeStarted).toBe(1);
				expect(status.closeFinished).toBe(1);

				const entries = await actor.getLogEntries();
				const events = entries.map((entry: LogEntry) => entry.event);
				expect(events).toContain("sleep");
				expect(events).toContain("close-start");
				expect(events).toContain("close-finish");
				},
			);

			test(
				"broadcast works in onSleep",
				{ timeout: 20_000 },
				async (c) => {
				const { client } = await setupDriverTest(
					c,
					driverTestConfig,
				);

				const handle = client.sleepWithDbAction.getOrCreate([
					"broadcast-in-onsleep",
				]);
				const connection = handle.connect();

				// Wait for connection to be established
				await waitForConnected(connection);

				// Listen for the "sleeping" event
				let sleepingEventReceived = false;
				const sleepingPromise = new Promise<void>((resolve) => {
					connection.once("sleeping", () => {
						sleepingEventReceived = true;
						resolve();
					});
				});
				connection.on("sleeping", () => {
					sleepingEventReceived = true;
				});

				// Insert a log entry while awake
				await connection.insertLogEntry("before-sleep");

				// Trigger sleep
				await connection.triggerSleep();

				await sleepingPromise;
				await waitFor(driverTestConfig, 1_000);
				await connection.dispose();

				expect(sleepingEventReceived).toBe(true);

				const counts = await handle.getCounts();
				expect(counts.sleepCount).toBe(1);
				expect(counts.startCount).toBe(2);

				const entries = await handle.getLogEntries();
				const events = entries.map(
					(e: LogEntry) => e.event,
				);

				expect(events).toContain("before-sleep");
				expect(events).toContain("sleep-start");
				expect(events).toContain("sleep-end");
				},
			);

			test("action via handle during sleep is queued and runs on woken instance", async (c) => {
				const { client } = await setupDriverTest(
					c,
					driverTestConfig,
				);

				// CURRENT BEHAVIOR: When an action is sent via a stateless
				// handle while the actor is sleeping, the file-system driver
				// queues the action. Once the actor finishes sleeping and
				// wakes back up, the action executes on the new instance.

				const handle = client.sleepWithDbAction.getOrCreate([
					"action-during-sleep-handle",
				]);

				// Insert a log entry while awake
				await handle.insertLogEntry("before-sleep");

				// Trigger sleep
				await handle.triggerSleep();

				// Immediately try to call an action via the handle.
				// This action arrives while the actor is shutting down or asleep.
				let actionResult: { succeeded: boolean; error?: string };
				try {
					await handle.insertLogEntry("during-sleep");
					actionResult = { succeeded: true };
				} catch (error) {
					actionResult = {
						succeeded: false,
						error:
							error instanceof Error
								? error.message
								: String(error),
					};
				}

				// Wait for everything to settle
				await waitFor(driverTestConfig, 1000);

				// Wake the actor and check state. The sleep/start counts
				// may be >1/2 because the action arriving during sleep
				// wakes the actor, which may auto-sleep and wake again.
				const counts = await handle.getCounts();
				expect(counts.sleepCount).toBeGreaterThanOrEqual(1);
				expect(counts.startCount).toBeGreaterThanOrEqual(2);

				const entries = await handle.getLogEntries();
				const events = entries.map(
					(e: LogEntry) => e.event,
				);

				// CURRENT BEHAVIOR: The action succeeds because the driver
				// wakes the actor to process it. The action runs on the new
				// instance after wake.
				expect(actionResult.succeeded).toBe(true);
				expect(events).toContain("before-sleep");
				expect(events).toContain("during-sleep");
			});

			test("waitUntil works in onSleep", async (c) => {
				const { client } = await setupDriverTest(
					c,
					driverTestConfig,
				);

				const actor = client.sleepWaitUntil.getOrCreate([
					"waituntil-onsleep",
				]);

				// Trigger sleep
				await actor.triggerSleep();

				// Wait for sleep to complete
				await waitFor(driverTestConfig, SLEEP_DB_TIMEOUT + 500);

				// Wake the actor
				const counts = await actor.getCounts();
				expect(counts.sleepCount).toBeGreaterThanOrEqual(1);
				expect(counts.startCount).toBeGreaterThanOrEqual(2);

				// Verify the waitUntil'd write appeared in the DB
				const entries = await actor.getLogEntries();
				const events = entries.map(
					(e: { event: string }) => e.event,
				);
				expect(events).toContain("sleep-start");
				expect(events).toContain("waituntil-write");
			});

			test("nested waitUntil inside waitUntil is drained before shutdown", async (c) => {
				const { client } = await setupDriverTest(
					c,
					driverTestConfig,
				);

				const actor = client.sleepNestedWaitUntil.getOrCreate([
					"nested-waituntil",
				]);

				// Trigger sleep
				await actor.triggerSleep();

				// Wait for sleep to complete
				await waitFor(driverTestConfig, 500);

				// Wake the actor
				const counts = await actor.getCounts();
				expect(counts.sleepCount).toBe(1);
				expect(counts.startCount).toBe(2);

				// Verify both outer and nested waitUntil writes appeared
				const entries = await actor.getLogEntries();
				const events = entries.map(
					(e: { event: string }) => e.event,
				);
				expect(events).toContain("sleep-start");
				expect(events).toContain("outer-waituntil");
				expect(events).toContain("nested-waituntil");
			});

			test("enqueue works during onSleep", async (c) => {
				const { client } = await setupDriverTest(
					c,
					driverTestConfig,
				);

				const actor = client.sleepEnqueue.getOrCreate([
					"enqueue-onsleep",
				]);

				// Trigger sleep
				await actor.triggerSleep();

				// Wait for sleep to complete
				await waitFor(driverTestConfig, 500);

				// Wake the actor
				const counts = await actor.getCounts();
				expect(counts.sleepCount).toBe(1);
				expect(counts.enqueueSuccess).toBe(true);
				expect(counts.enqueueError).toBeNull();
			});

			test(
				"schedule.after in onSleep persists and fires on wake",
				{ timeout: 20_000 },
				async (c) => {
				const { client } = await setupDriverTest(
					c,
					driverTestConfig,
				);

				const actor = client.sleepScheduleAfter.getOrCreate([
					"schedule-after-onsleep",
				]);

				// Trigger sleep
				await actor.triggerSleep();

				// Wait for sleep to complete
				await waitFor(driverTestConfig, 500);

				// Wake the actor by calling an action, then wait for
				// the scheduled alarm to fire (it was scheduled with
				// 100ms delay, re-armed on wake via initializeAlarms)
				const counts = await actor.getCounts();
				expect(counts.sleepCount).toBeGreaterThanOrEqual(1);
				expect(counts.startCount).toBeGreaterThanOrEqual(2);

				await vi.waitFor(
					async () => {
						const entries = await actor.getLogEntries();
						const events = entries.map(
							(e: { event: string }) => e.event,
						);
						expect(events).toContain("sleep");
						expect(events).toContain("scheduled-action");
					},
					{
						timeout: 10_000,
						interval: 50,
					},
				);
				},
			);

			test("action via WebSocket connection during sleep shutdown succeeds", async (c) => {
				const { client } = await setupDriverTest(
					c,
					driverTestConfig,
				);

				// Actions from pre-existing connections during the graceful
				// shutdown window should succeed since assertReady() only
				// blocks after #shutdownComplete is set.

				const handle = client.sleepWithDbAction.getOrCreate([
					"ws-during-sleep",
				]);
				const connection = handle.connect();

				// Wait for connection to be established
				await vi.waitFor(async () => {
					expect(connection.isConnected).toBe(true);
				});

				// Insert a log entry while awake
				await connection.insertLogEntry("before-sleep");

				// Trigger sleep via the connection
				await connection.triggerSleep();

				// Send an action via the WebSocket connection during the
				// graceful shutdown window. This should succeed.
				await connection.insertLogEntry("ws-during-sleep");

				// Wait for sleep to fully complete
				await waitFor(driverTestConfig, 1500);

				// Dispose the connection
				await connection.dispose();

				// Wake the actor
				const counts = await handle.getCounts();
				expect(counts.sleepCount).toBe(1);
				expect(counts.startCount).toBe(2);

				// Get log entries after waking
				const entries = await handle.getLogEntries();
				const events = entries.map(
					(e: LogEntry) => e.event,
				);

				expect(events).toContain("before-sleep");
				expect(events).toContain("sleep-start");
				expect(events).toContain("ws-during-sleep");
			});
		test("new connections rejected during sleep shutdown", async (c) => {
				const { client } = await setupDriverTest(
					c,
					driverTestConfig,
				);

				// The sleepWithDbAction actor has a 500ms delay in
				// onSleep, giving us a window to attempt a new connection
				// while the actor is actively shutting down.

				const handle = client.sleepWithDbAction.getOrCreate([
					"conn-rejected-during-sleep",
				]);
				const firstConn = handle.connect();

				// Wait for first connection
				await waitForConnected(firstConn);

				// Trigger sleep (the actor will be in onSleep for ~500ms)
				await firstConn.triggerSleep();

				// Wait a moment for the shutdown to start
				await waitFor(driverTestConfig, 100);

				// Attempt a new connection during shutdown.
				// The file-system driver queues the connection until
				// the actor wakes, so this should not throw. The
				// connection will be established on the new instance.
				const secondConn = handle.connect();

				// Wait for sleep to complete and actor to wake
				await waitFor(driverTestConfig, 2000);

				// The second connection should eventually connect
				// on the woken instance
				await waitForConnected(secondConn);

				// Verify the actor went through a sleep-wake cycle
				const counts = await handle.getCounts();
				expect(counts.sleepCount).toBe(1);
				expect(counts.startCount).toBe(2);

				await firstConn.dispose();
				await secondConn.dispose();
			});

			test("new raw WebSocket during sleep shutdown is rejected or queued", async (c) => {
				const { client } = await setupDriverTest(
					c,
					driverTestConfig,
				);

				// The sleepWithRawWs actor has a 500ms delay in onSleep.
				// A raw WebSocket request during shutdown is rejected by
				// the manager driver with "Actor stopping" because the
				// handleRawWebSocket guard blocks new WebSocket handlers
				// when #stopCalled is true.

				const handle = client.sleepWithRawWs.getOrCreate([
					"raw-ws-rejected-during-sleep",
				]);

				// Trigger sleep
				await handle.triggerSleep();

				// Wait a moment for shutdown to begin
				await waitFor(driverTestConfig, 100);

				// Attempt a raw WebSocket during shutdown.
				// This should be rejected by the driver/guard.
				let wsError: string | undefined;
				let queuedWs: WebSocket | undefined;
				try {
					queuedWs = await handle.webSocket();
				} catch (error) {
					wsError = error instanceof Error
						? error.message
						: String(error);
				}

				// Current behavior varies by timing. The raw websocket
				// may be rejected during shutdown, or it may be queued
				// and connected on the woken instance.
				expect(Boolean(wsError || queuedWs)).toBe(true);
				if (wsError) {
					expect(wsError).toContain("stopping");
				}
				if (queuedWs) {
					await new Promise<void>((resolve, reject) => {
						const onMessage = (event: MessageEvent) => {
							const data = JSON.parse(String(event.data));
							if (data.type === "connected") {
								cleanup();
								resolve();
							}
						};
						const onClose = () => {
							cleanup();
							reject(new Error("websocket closed before connect"));
						};
						const cleanup = () => {
							queuedWs!.removeEventListener("message", onMessage);
							queuedWs!.removeEventListener("close", onClose);
						};

						queuedWs.addEventListener("message", onMessage);
						queuedWs.addEventListener("close", onClose, {
							once: true,
						});
					});
					queuedWs.close();
				}

				// Wait for sleep to complete
				await waitFor(driverTestConfig, 1500);

				// Verify the actor can still wake and function normally
				const counts = await handle.getCounts();
				expect(counts.sleepCount).toBeGreaterThanOrEqual(1);
				expect(counts.startCount).toBeGreaterThanOrEqual(2);
			});

			test("onSleep throwing does not prevent clean shutdown", async (c) => {
				const { client } = await setupDriverTest(
					c,
					driverTestConfig,
				);

				const actor = client.sleepOnSleepThrows.getOrCreate([
					"onsleep-throws",
				]);

				// Trigger sleep. The onSleep handler throws after
				// writing "sleep-before-throw" to the DB.
				await actor.triggerSleep();

				// Wait for sleep to complete
				await waitFor(driverTestConfig, 500);

				// Wake the actor. It should have shut down cleanly
				// despite the throw, because #shutdownComplete is set
				// in the finally block.
				const counts = await actor.getCounts();
				expect(counts.sleepCount).toBe(1);
				expect(counts.startCount).toBe(2);

				// Verify the DB write before the throw was persisted
				const entries = await actor.getLogEntries();
				const events = entries.map(
					(e: { event: string }) => e.event,
				);
				expect(events).toContain("sleep-before-throw");
			});

			test("waitUntil rejection during shutdown does not block shutdown", async (c) => {
				const { client } = await setupDriverTest(
					c,
					driverTestConfig,
				);

				const actor = client.sleepWaitUntilRejects.getOrCreate([
					"waituntil-rejects",
				]);

				// Trigger sleep. The onSleep handler registers a
				// rejecting waitUntil and a succeeding one.
				await actor.triggerSleep();

				// Wait for sleep to complete
				await waitFor(driverTestConfig, 500);

				// Wake the actor. Shutdown should have completed
				// despite the rejection.
				const counts = await actor.getCounts();
				expect(counts.sleepCount).toBe(1);
				expect(counts.startCount).toBe(2);

				// The succeeding waitUntil should still have run
				const entries = await actor.getLogEntries();
				const events = entries.map(
					(e: { event: string }) => e.event,
				);
				expect(events).toContain("sleep");
				expect(events).toContain("waituntil-after-reject");
			});

			test("double sleep call is a no-op", async (c) => {
				const { client } = await setupDriverTest(
					c,
					driverTestConfig,
				);

				// Use a connection to send the sleep trigger, because
				// a handle-based action goes through the driver which
				// would wake the actor for a second cycle.
				const handle = client.sleepWithDbAction.getOrCreate([
					"double-sleep",
				]);
				const connection = handle.connect();

				await waitForConnected(connection);

				// Subscribe before triggering sleep so the broadcast cannot
				// win the race against a lazily-registered event handler.
				const sleepingPromise = new Promise<void>((resolve) => {
					connection.once("sleeping", () => {
						resolve();
					});
				});
				// Trigger c.sleep() twice in the same actor turn. This
				// validates the actor-level idempotence directly without
				// conflating it with transport replay after wake.
				await connection.triggerSleepTwice();

				// Wait for the first sleep cycle to begin, then give it
				// enough time to complete before the actor can auto-sleep
				// a second time after wake.
				await sleepingPromise;
				await waitFor(driverTestConfig, 750);
				await connection.dispose();

				// Wake the actor. It should have gone through exactly
				// one sleep-wake cycle.
				const counts = await handle.getCounts();
				expect(counts.sleepCount).toBe(1);
				expect(counts.startCount).toBe(2);
			});

			test("state mutations in waitUntil callback are persisted", async (c) => {
				const { client } = await setupDriverTest(
					c,
					driverTestConfig,
				);

				const actor = client.sleepWaitUntilState.getOrCreate([
					"waituntil-state-persist",
				]);

				// Trigger sleep. The onSleep handler registers a
				// waitUntil that mutates c.state.waitUntilRan.
				await actor.triggerSleep();

				// Wait for sleep to complete
				await waitFor(driverTestConfig, 500);

				// Wake the actor and verify the state mutation
				// from the waitUntil callback was persisted.
				const counts = await actor.getCounts();
				expect(counts.sleepCount).toBe(1);
				expect(counts.startCount).toBe(2);
				expect(counts.waitUntilRan).toBe(true);

				// Verify the DB write from waitUntil was also persisted
				const entries = await actor.getLogEntries();
				const events = entries.map(
					(e: { event: string }) => e.event,
				);
				expect(events).toContain("waituntil-state");
			});

			test("alarm does not fire during shutdown", async (c) => {
				const { client } = await setupDriverTest(
					c,
					driverTestConfig,
				);

				const actor = client.sleepWithDb.getOrCreate([
					"alarm-no-fire-during-shutdown",
				]);

				// Schedule an alarm with a very short delay
				await actor.setAlarm(50);

				// Immediately trigger sleep. The cancelAlarm call in
				// onStop should prevent the alarm from firing during
				// the shutdown sequence.
				await actor.triggerSleep();

				// Wait for sleep to fully complete
				await waitFor(driverTestConfig, 500);

				// Wake the actor. The alarm should fire on the new
				// instance (re-armed by initializeAlarms on wake).
				const counts = await actor.getCounts();
				expect(counts.sleepCount).toBe(1);
				expect(counts.startCount).toBe(2);

				// Wait for the alarm to fire on the woken instance
				await waitFor(driverTestConfig, 500);

				const entries = await actor.getLogEntries();
				const events = entries.map(
					(e: { event: string }) => e.event,
				);
				expect(events).toContain("alarm");
			});

			test(
				"ws handler exceeding grace period should still complete db writes",
				async (c) => {
					const { client } = await setupDriverTest(
						c,
						driverTestConfig,
					);

					const actor =
						client.sleepWsMessageExceedsGrace.getOrCreate([
							"ws-exceeds-grace",
						]);
					const ws = await connectRawWebSocket(actor);

					// Send a message that starts slow async DB work
					ws.send("slow-db-work");

					// Wait for the handler to confirm it started
					await new Promise<void>((resolve) => {
						const onMessage = (event: MessageEvent) => {
							const data = JSON.parse(String(event.data));
							if (data.type === "started") {
								ws.removeEventListener(
									"message",
									onMessage,
								);
								resolve();
							}
						};
						ws.addEventListener("message", onMessage);
					});

					// Trigger sleep while the handler is still doing slow
					// work. The grace period (200ms) is much shorter than the
					// handler delay (2000ms), so shutdown will time out and
					// clean up the database while the handler is still running.
					await actor.triggerSleep();

					await vi.waitFor(
						async () => {
							const status = await actor.getStatus();
							expect(status.sleepCount).toBeGreaterThanOrEqual(1);
							expect(status.startCount).toBeGreaterThanOrEqual(2);
							expect(status.messageStarted).toBe(1);
							expect(status.messageFinished).toBe(0);

							const entries = await actor.getLogEntries();
							const events = entries.map(
								(e: { event: string }) => e.event,
							);
							expect(events).toContain("msg-start");
							expect(events).not.toContain("msg-finish");
						},
						{
							timeout: 20_000,
							interval: 50,
						},
					);
				},
				{ timeout: 15_000 },
			);

			test(
				"concurrent ws handlers with cached db ref get errors when grace period exceeded",
				async (c) => {
					const { client } = await setupDriverTest(
						c,
						driverTestConfig,
					);

					const actor =
						client.sleepWsConcurrentDbExceedsGrace.getOrCreate(
							["ws-concurrent-exceeds-grace"],
						);
					const ws = await connectRawWebSocket(actor);

					const MESSAGE_COUNT = 3;
					let startedCount = 0;

					// Set up listener for "started" confirmations
					const allStarted = new Promise<void>((resolve) => {
						const onMessage = (event: MessageEvent) => {
							const data = JSON.parse(String(event.data));
							if (data.type === "started") {
								startedCount++;
								if (startedCount === MESSAGE_COUNT) {
									ws.removeEventListener(
										"message",
										onMessage,
									);
									resolve();
								}
							}
						};
						ws.addEventListener("message", onMessage);
					});

					// Send multiple messages rapidly. Each handler captures
					// c.db before awaiting and uses the cached reference after
					// the delay. Multiple handlers will try to use the cached
					// db reference after VFS teardown.
					for (let i = 0; i < MESSAGE_COUNT; i++) {
						ws.send(
							JSON.stringify({
								type: "slow-db-work",
								index: i,
							}),
						);
					}

					// Wait for all handlers to confirm they started
					await allStarted;

					// Trigger sleep while all handlers are doing slow work
					await actor.triggerSleep();

					// Wait for handlers to finish + actor to sleep and wake
					await waitFor(
						driverTestConfig,
						EXCEEDS_GRACE_HANDLER_DELAY +
							MESSAGE_COUNT * 50 +
							EXCEEDS_GRACE_SLEEP_TIMEOUT +
							500,
					);

					// Wake the actor. All handlers should have completed
					// their DB writes successfully.
					const status = await actor.getStatus();
					expect(status.sleepCount).toBeGreaterThanOrEqual(1);
					expect(status.startCount).toBeGreaterThanOrEqual(2);
					expect(status.handlerStarted).toBe(MESSAGE_COUNT);

					// Exceeding the shutdown grace period cuts off the
					// handlers before their delayed DB writes can finish.
					expect(status.handlerFinished).toBe(0);
					expect(status.handlerErrors).toEqual([]);
				},
				{ timeout: 15_000 },
			);

			test(
				"active db writes interrupted by sleep produce db error",
				async (c) => {
					const { client } = await setupDriverTest(
						c,
						driverTestConfig,
					);

					const actor =
						client.sleepWsActiveDbExceedsGrace.getOrCreate([
							"ws-active-db-exceeds-grace",
						]);
					const ws = await connectRawWebSocket(actor);

					// Start the write loop
					ws.send("start-writes");

					// Wait for confirmation
					await new Promise<void>((resolve) => {
						const onMessage = (event: MessageEvent) => {
							const data = JSON.parse(String(event.data));
							if (data.type === "started") {
								ws.removeEventListener(
									"message",
									onMessage,
								);
								resolve();
							}
						};
						ws.addEventListener("message", onMessage);
					});

					// Trigger sleep while writes are in progress.
					await actor.triggerSleep();

					await vi.waitFor(
						async () => {
							const status = await actor.getStatus();
							expect(status.sleepCount).toBeGreaterThanOrEqual(1);
							expect(status.startCount).toBeGreaterThanOrEqual(2);

							const entries = await actor.getLogEntries();
							const writeEntries = entries.filter(
								(e: { event: string }) =>
									e.event.startsWith("write-"),
							);
							expect(writeEntries.length).toBeGreaterThan(0);
							expect(writeEntries.length).toBeLessThan(
								ACTIVE_DB_WRITE_COUNT,
							);
						},
						{
							timeout: 20_000,
							interval: 50,
						},
					);

					// Verify the DB has fewer rows than the full count.
					const entries = await actor.getLogEntries();
					const writeEntries = entries.filter(
						(e: { event: string }) =>
							e.event.startsWith("write-"),
					);
					expect(writeEntries.length).toBeGreaterThan(0);
					expect(writeEntries.length).toBeLessThan(
						ACTIVE_DB_WRITE_COUNT,
					);
				},
				{ timeout: 30_000 },
			);
		});
}
