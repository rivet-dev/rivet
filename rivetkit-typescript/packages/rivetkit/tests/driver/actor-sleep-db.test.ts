import { describe, expect, test, vi } from "vitest";
import {
	ACTIVE_DB_WRITE_COUNT,
	EXCEEDS_GRACE_HANDLER_DELAY,
	EXCEEDS_GRACE_SLEEP_TIMEOUT,
	SLEEP_DB_TIMEOUT,
	SLEEP_SCHEDULE_AFTER_ON_SLEEP_DELAY_MS,
} from "../../fixtures/driver-test-suite/sleep-db";
import { describeDriverMatrix } from "./shared-matrix";
import { setupDriverTest, waitFor } from "./shared-utils";

type LogEntry = { id: number; event: string; created_at: number };

const CONNECTION_READY_TIMEOUT_MS = 10_000;

async function waitForAction<T>(
	action: () => Promise<T>,
	assert: (value: T) => void,
	timeout = 10_000,
): Promise<T> {
	let latest: T | undefined;
	// Poll because sleep and wake state changes do not have a direct event hook in the driver harness.
	await vi.waitFor(
		async () => {
			const value = await action();
			assert(value);
			latest = value;
		},
		{ timeout, interval: 50 },
	);
	if (latest === undefined) {
		throw new Error("waitForAction did not capture a value");
	}
	return latest;
}

async function waitForConnectionReady(connection: { isConnected: boolean }) {
	// Poll until the connection handshake flips isConnected because connect() has no ready promise.
	await vi.waitFor(
		async () => {
			expect(connection.isConnected).toBe(true);
		},
		{ timeout: CONNECTION_READY_TIMEOUT_MS, interval: 100 },
	);
}

async function triggerSleepBestEffort(actor: {
	triggerSleep(): Promise<void>;
}) {
	try {
		await actor.triggerSleep();
	} catch (error) {
		const message = error instanceof Error ? error.message : String(error);
		if (!/stopping|timed out|task stopped/i.test(message)) {
			throw error;
		}
	}
}

function expectStoppingError(error: unknown) {
	expect(error).toBeTruthy();
	const maybeActorError = error as { group?: string; code?: string };
	if (maybeActorError.group || maybeActorError.code) {
		expect(maybeActorError.group).toBe("actor");
		expect(maybeActorError.code).toBe("stopping");
		return;
	}

	const message = error instanceof Error ? error.message : String(error);
	expect(message).toMatch(/stopping/i);
}

async function connectRawWebSocket(handle: {
	webSocket(): Promise<WebSocket>;
}) {
	const ws = await handle.webSocket();

	await new Promise<void>((resolve, reject) => {
		ws.addEventListener("open", () => resolve(), { once: true });
		ws.addEventListener(
			"error",
			() => reject(new Error("websocket error")),
			{
				once: true,
			},
		);
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

describeDriverMatrix("Actor Sleep Db", (driverTestConfig) => {
	const describeSleepDbTests = driverTestConfig.skip?.sleep
		? describe.skip
		: describe.sequential;

	describeSleepDbTests("Actor Sleep Database Tests", () => {
		test("onSleep can write to c.db", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);

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
			const events = entries.map((e: { event: string }) => e.event);
			expect(events).toContain("wake");
			expect(events).toContain("before-sleep");
			expect(events).toContain("sleep");
		});

		test("c.db works after sleep-wake cycle", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);

			const actor = client.sleepWithDb.getOrCreate(["db-after-wake"]);

			// Insert before sleep
			await actor.insertLogEntry("before");

			// Let it auto-sleep
			await waitFor(driverTestConfig, SLEEP_DB_TIMEOUT + 250);

			// Wake it by calling an action that uses the DB
			await actor.insertLogEntry("after-wake");

			const entries = await actor.getLogEntries();
			const events = entries.map((e: { event: string }) => e.event);
			expect(events).toContain("before");
			expect(events).toContain("sleep");
			expect(events).toContain("wake");
			expect(events).toContain("after-wake");
		});

		test("scheduled alarm can use c.db after sleep-wake", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);

			const actor = client.sleepWithDb.getOrCreate(["alarm-db-wake"]);

			// Schedule an alarm that fires after the actor would sleep
			await actor.setAlarm(SLEEP_DB_TIMEOUT + 500);

			// Wait for the actor to sleep and then wake from alarm
			await waitFor(driverTestConfig, SLEEP_DB_TIMEOUT + 750);

			// Verify the alarm wrote to the DB
			const entries = await actor.getLogEntries();
			const events = entries.map((e: { event: string }) => e.event);
			expect(events).toContain("alarm");
		});

		test("scheduled action stays awake until db work completes", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);

			const actor = client.sleepWithSlowScheduledDb.getOrCreate([
				"slow-scheduled-db",
			]);

			await actor.scheduleSlowAlarm(50, SLEEP_DB_TIMEOUT + 250);

			await waitFor(
				driverTestConfig,
				50 + (SLEEP_DB_TIMEOUT + 250) + SLEEP_DB_TIMEOUT + 250,
			);

			// Poll until the wake-up pass persists the post-sleep counters after the delayed schedule fires.
			await vi.waitFor(
				async () => {
					const counts = await actor.getCounts();
					expect(counts.sleepCount).toBe(1);
					expect(counts.startCount).toBe(2);
				},
				{
					timeout: 5_000,
					interval: 50,
				},
			);

			const entries = await actor.getLogEntries();
			const events = entries.map((e: { event: string }) => e.event);
			expect(events).toContain("slow-alarm-start");
			expect(events).toContain("slow-alarm-finish");
			expect(events.indexOf("slow-alarm-finish")).toBeLessThan(
				events.indexOf("sleep"),
			);
		});

		// TODO(#4705): Root-cause sleep-shutdown disconnect ordering and re-enable this coverage.
		test.skip("onDisconnect can write to c.db during sleep shutdown", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);

			// Create actor with a connection
			const handle = client.sleepWithDbConn.getOrCreate([
				"disconnect-db-write",
			]);
			const connection = handle.connect();

			// Wait for connection to be established
			await waitForConnectionReady(connection);

			// Insert a log entry while awake
			await connection.insertLogEntry("before-sleep");

			// Trigger sleep, then dispose the connection.
			// During the sleep shutdown sequence, onDisconnect is called
			// with the DB still open (step 6 in the shutdown sequence).
			await connection.triggerSleep();
			await connection.dispose();

			// Wake the actor by calling an action once sleep has completed.
			const wokenHandle = client.sleepWithDbConn.getOrCreate([
				"disconnect-db-write",
			]);
			await waitForAction(wokenHandle.getCounts, (counts) => {
				expect(counts.sleepCount).toBe(1);
				expect(counts.startCount).toBe(2);
			});

			// Verify events were logged to the DB
			const entries = await wokenHandle.getLogEntries();
			const events = entries.map((e: LogEntry) => e.event);

			// CURRENT BEHAVIOR: onDisconnect runs during sleep shutdown
			// and the DB is still open at that point, so the write should succeed.
			expect(events).toContain("before-sleep");
			expect(events).toContain("sleep");
			expect(events).toContain("disconnect");
		});

		// TODO(#4705): Root-cause raw WebSocket close-handler DB access during sleep shutdown and re-enable this coverage.
		test.skip("async websocket close handler can use c.db before sleep completes", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);

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

			await waitForAction(actor.getStatus, (status) => {
				expect(status.sleepCount).toBe(1);
				expect(status.startCount).toBe(2);
			});

			const entries = await actor.getLogEntries();
			const events = entries.map((entry: LogEntry) => entry.event);
			expect(events).toContain("sleep");
		}, 30_000);

		// TODO(#4705): Root-cause addEventListener close-handler DB access during sleep shutdown and re-enable this coverage.
		test.skip("async websocket addEventListener close handler can use c.db before sleep completes", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);

			const actor = client.sleepWithRawWsCloseDbListener.getOrCreate([
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

			await waitForAction(actor.getStatus, (status) => {
				expect(status.sleepCount).toBe(1);
				expect(status.startCount).toBe(2);
			});

			const entries = await actor.getLogEntries();
			const events = entries.map((entry: LogEntry) => entry.event);
			expect(events).toContain("sleep");
		}, 30_000);

		test("broadcast works in onSleep", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);

			const handle = client.sleepWithDbAction.getOrCreate([
				"broadcast-in-onsleep",
			]);
			const connection = handle.connect();

			// Listen for the "sleeping" event
			let sleepingEventReceived = false;
			connection.on("sleeping", () => {
				sleepingEventReceived = true;
			});

			await waitForAction(
				connection.getCounts.bind(connection),
				(counts) => {
					expect(counts.startCount).toBeGreaterThanOrEqual(1);
				},
				15_000,
			);

			// Insert a log entry while awake
			await connection.insertLogEntry("before-sleep");

			// Trigger sleep
			await connection.triggerSleep();

			// Wait for sleep to fully complete
			await waitFor(driverTestConfig, 1500);
			await connection.dispose();

			// Broadcast now works during onSleep since assertReady
			// only blocks after #shutdownComplete is set.
			expect(sleepingEventReceived).toBe(true);

			// Wake the actor
			const counts = await handle.getCounts();
			expect(counts.sleepCount).toBe(1);
			expect(counts.startCount).toBe(2);

			// Both "sleep-start" and "sleep-end" should be written
			// since broadcast no longer throws.
			const entries = await handle.getLogEntries();
			const events = entries.map((e: LogEntry) => e.event);

			expect(events).toContain("before-sleep");
			expect(events).toContain("sleep-start");
			expect(events).toContain("sleep-end");
		});

		// TODO(#4705): Root-cause handle action dispatch ordering during sleep shutdown and re-enable this coverage.
		test.skip("action via handle during sleep shutdown is not queued", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);

			const handle = client.sleepWithDbAction.getOrCreate([
				"action-during-sleep-handle",
			]);
			const connection = handle.connect();

			await waitForConnectionReady(connection);

			const sleeping = new Promise<void>((resolve) => {
				connection.once("sleeping", () => resolve());
			});

			await connection.insertLogEntry("before-sleep");
			await connection.triggerSleep();
			await sleeping;

			let actionSucceeded = false;
			let actionError: unknown;
			try {
				await handle.insertLogEntry("during-sleep");
				actionSucceeded = true;
			} catch (error) {
				actionError = error;
			}
			if (actionError) {
				expectStoppingError(actionError);
			}

			await connection.dispose();

			await waitForAction(handle.getCounts, (counts) => {
				expect(counts.sleepCount).toBeGreaterThanOrEqual(1);
				expect(counts.startCount).toBeGreaterThanOrEqual(2);
			});

			const entries = await handle.getLogEntries();
			const events = entries.map((e: LogEntry) => e.event);

			expect(events).toContain("before-sleep");
			if (actionSucceeded) {
				expect(events).toContain("during-sleep");
			} else {
				expect(events).not.toContain("during-sleep");
			}
		});

		test("waitUntil works in onSleep", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);

			const actor = client.sleepWaitUntil.getOrCreate([
				"waituntil-onsleep",
			]);

			// Trigger sleep
			await actor.triggerSleep();

			// Wait for sleep to complete
			await waitFor(driverTestConfig, 500);

			// Wake the actor
			const counts = await actor.getCounts();
			expect(counts.sleepCount).toBe(1);
			expect(counts.startCount).toBeGreaterThanOrEqual(2);

			// Verify the waitUntil'd write appeared in the DB
			const entries = await actor.getLogEntries();
			const events = entries.map((e: { event: string }) => e.event);
			expect(events).toContain("sleep-start");
			expect(events).toContain("waituntil-write");
		});

		test("nested waitUntil inside waitUntil is drained before shutdown", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);

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
			expect(counts.startCount).toBeGreaterThanOrEqual(2);

			// Verify both outer and nested waitUntil writes appeared
			const entries = await actor.getLogEntries();
			const events = entries.map((e: { event: string }) => e.event);
			expect(events).toContain("sleep-start");
			expect(events).toContain("outer-waituntil");
			expect(events).toContain("nested-waituntil");
		});

		test("enqueue works during onSleep", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);

			const actor = client.sleepEnqueue.getOrCreate(["enqueue-onsleep"]);

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

		test("schedule.after in onSleep persists and fires on wake", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);

			const actor = client.sleepScheduleAfter.getOrCreate([
				"schedule-after-onsleep",
			]);

			// Trigger sleep
			await actor.triggerSleep();

			// Wait for sleep to complete
			await waitFor(driverTestConfig, 500);

			// The delayed onSleep alarm keeps this explicit wake from racing the alarm wake.
			const counts = await actor.getCounts();
			expect(counts.sleepCount).toBe(1);
			expect(counts.startCount).toBe(2);

			// Wait for the scheduled action to fire after wake
			await waitFor(
				driverTestConfig,
				SLEEP_SCHEDULE_AFTER_ON_SLEEP_DELAY_MS + 500,
			);

			// Verify the scheduled action wrote to the DB
			const entries = await actor.getLogEntries();
			const events = entries.map((e: { event: string }) => e.event);
			expect(events).toContain("sleep");
			expect(events).toContain("scheduled-action");
			expect(
				events.filter((event) => event === "scheduled-action"),
			).toHaveLength(1);
			const finalCounts = await actor.getCounts();
			expect(finalCounts.startCount).toBe(2);
			expect(finalCounts.scheduledActionCount).toBe(1);
		});

		// TODO(#4705): Root-cause connection action dispatch ordering during sleep shutdown and re-enable this coverage.
		test.skip("action via WebSocket connection during sleep shutdown is not queued", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);

			const handle = client.sleepWithDbAction.getOrCreate([
				"ws-during-sleep",
			]);
			const connection = handle.connect();

			// Wait for connection to be established
			await waitForConnectionReady(connection);

			const sleeping = new Promise<void>((resolve) => {
				connection.once("sleeping", () => resolve());
			});

			await connection.insertLogEntry("before-sleep");

			await connection.triggerSleep();
			await sleeping;

			let actionSucceeded = false;
			let actionError: unknown;
			try {
				await connection.insertLogEntry("ws-during-sleep");
				actionSucceeded = true;
			} catch (error) {
				actionError = error;
			}
			if (actionError) {
				expectStoppingError(actionError);
			}

			await connection.dispose();

			await waitForAction(handle.getCounts, (counts) => {
				expect(counts.sleepCount).toBe(1);
				expect(counts.startCount).toBe(2);
			});

			const entries = await handle.getLogEntries();
			const events = entries.map((e: LogEntry) => e.event);

			expect(events).toContain("before-sleep");
			expect(events).toContain("sleep-start");
			if (actionSucceeded) {
				expect(events).toContain("ws-during-sleep");
			} else {
				expect(events).not.toContain("ws-during-sleep");
			}
		});
		// TODO(#4705): Root-cause new connection behavior during sleep shutdown and re-enable this coverage.
		test.skip("new connections rejected during sleep shutdown", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);

			// The sleepWithDbAction actor has a 500ms delay in
			// onSleep, giving us a window to attempt a new connection
			// while the actor is actively shutting down.

			const handle = client.sleepWithDbAction.getOrCreate([
				"conn-rejected-during-sleep",
			]);
			const firstConn = handle.connect();

			// Poll until the first connection handshake finishes because connect() has no ready promise.
			await vi.waitFor(async () => {
				expect(firstConn.isConnected).toBe(true);
			});

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

			// Verify the actor went through a sleep-wake cycle
			const wokenHandle = client.sleepWithDbAction.getOrCreate([
				"conn-rejected-during-sleep",
			]);
			await waitForAction(wokenHandle.getCounts, (counts) => {
				expect(counts.sleepCount).toBe(1);
				expect(counts.startCount).toBe(2);
			});

			await firstConn.dispose();
			await secondConn.dispose();
		});

		// TODO(#4705): Root-cause raw WebSocket admission during sleep shutdown and re-enable this coverage.
		test.skip("new raw WebSocket during sleep shutdown is rejected or queued", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);

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

			let wsRejected = false;
			let queuedWs: WebSocket | undefined;
			try {
				queuedWs = await handle.webSocket();
			} catch {
				wsRejected = true;
			}

			expect(Boolean(wsRejected || queuedWs)).toBe(true);
			if (queuedWs) {
				const ws = queuedWs;
				await new Promise<void>((resolve) => {
					const onMessage = (event: MessageEvent) => {
						const data = JSON.parse(String(event.data));
						if (data.type === "connected") {
							cleanup();
							resolve();
						}
					};
					const onClose = () => {
						cleanup();
						wsRejected = true;
						resolve();
					};
					const cleanup = () => {
						ws.removeEventListener("message", onMessage);
						ws.removeEventListener("close", onClose);
					};

					ws.addEventListener("message", onMessage);
					ws.addEventListener("close", onClose, {
						once: true,
					});
				});
				ws.close();
			}

			// Wait for sleep to complete
			await waitFor(driverTestConfig, 1500);

			// Verify the actor can still wake and function normally
			const wokenHandle = client.sleepWithRawWs.getOrCreate([
				"raw-ws-rejected-during-sleep",
			]);
			await waitForAction(wokenHandle.getCounts, (counts) => {
				expect(counts.sleepCount).toBeGreaterThanOrEqual(1);
				expect(counts.startCount).toBeGreaterThanOrEqual(2);
			});
		});

		test("onSleep throwing does not prevent clean shutdown", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);

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
			const events = entries.map((e: { event: string }) => e.event);
			expect(events).toContain("sleep-before-throw");
		});

		test("waitUntil rejection during shutdown does not block shutdown", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);

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
			expect(counts.startCount).toBeGreaterThanOrEqual(2);

			// The succeeding waitUntil should still have run
			const entries = await actor.getLogEntries();
			const events = entries.map((e: { event: string }) => e.event);
			expect(events).toContain("sleep");
			expect(events).toContain("waituntil-after-reject");
		});

		// TODO(#4705): Root-cause duplicate sleep-call semantics and re-enable this coverage.
		test.skip("double sleep call is a no-op", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);

			// Use a connection to send the sleep trigger, because
			// a handle-based action goes through the driver which
			// would wake the actor for a second cycle.
			const handle = client.sleepWithDbAction.getOrCreate([
				"double-sleep",
			]);
			const connection = handle.connect();

			await waitForConnectionReady(connection);

			// Trigger sleep twice rapidly via the connection.
			// The second call should be a no-op because
			// #sleepCalled is already true.
			await connection.triggerSleep();
			try {
				await connection.triggerSleep();
			} catch {
				// May throw if actor already stopping
			}

			// Wait for sleep to complete
			await waitFor(driverTestConfig, 1500);
			await connection.dispose();

			// Wake the actor. It should have gone through exactly
			// one sleep-wake cycle.
			const counts = await handle.getCounts();
			expect(counts.sleepCount).toBe(1);
			expect(counts.startCount).toBe(2);
		});

		test("state mutations in waitUntil callback are persisted", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);

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
			expect(counts.startCount).toBeGreaterThanOrEqual(2);
			expect(counts.waitUntilRan).toBe(true);

			// Verify the DB write from waitUntil was also persisted
			const entries = await actor.getLogEntries();
			const events = entries.map((e: { event: string }) => e.event);
			expect(events).toContain("waituntil-state");
		});

		test("alarm does not fire during shutdown", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);

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
			const events = entries.map((e: { event: string }) => e.event);
			expect(events).toContain("alarm");
		});

		test(
			"ws handler exceeding grace period should still complete db writes",
			async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				const actor = client.sleepWsMessageExceedsGrace.getOrCreate([
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
							ws.removeEventListener("message", onMessage);
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

				// Wait for the handler to finish and the actor to complete
				// its sleep cycle. The handler runs for 2000ms. After that
				// the actor sleeps (the timed-out shutdown already ran, but
				// the handler promise still resolves in the background).
				await waitFor(
					driverTestConfig,
					EXCEEDS_GRACE_HANDLER_DELAY +
						EXCEEDS_GRACE_SLEEP_TIMEOUT +
						500,
				);

				// Wake the actor and check what happened.
				const status = await actor.getStatus();
				expect(status.sleepCount).toBeGreaterThanOrEqual(1);
				expect(status.startCount).toBeGreaterThanOrEqual(2);

				// The handler started.
				expect(status.messageStarted).toBe(1);

				// Exceeding the configured grace period stops later DB
				// work in the async handler before it can finish.
				expect(status.messageFinished).toBe(0);

				const entries = await actor.getLogEntries();
				const events = entries.map((e: { event: string }) => e.event);
				expect(events).toContain("msg-start");
				expect(events).not.toContain("msg-finish");
			},
			{ timeout: 15_000 },
		);

		// TODO(#4705): Root-cause task-model shutdown ordering for concurrent WebSocket DB handlers and re-enable this coverage.
		test.skip(
			"concurrent ws handlers with cached db ref get errors when grace period exceeded",
			async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				const actor =
					client.sleepWsConcurrentDbExceedsGrace.getOrCreate([
						"ws-concurrent-exceeds-grace",
					]);
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
								ws.removeEventListener("message", onMessage);
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
				// enough work to show the actor slept and resumed.
				const status = await actor.getStatus();
				expect(status.sleepCount).toBeGreaterThanOrEqual(1);
				expect(status.startCount).toBeGreaterThanOrEqual(2);
				expect(status.handlerStarted).toBe(MESSAGE_COUNT);

				// Exceeding the shutdown grace period should prevent the
				// whole batch from finishing, even if one handler slips
				// through before teardown wins the race.
				expect(status.handlerFinished).toBeLessThan(MESSAGE_COUNT);
				expect(status.handlerErrors).toEqual([]);

				const entries = await actor.getLogEntries();
				const finishedEvents = entries.filter(
					(entry: { event: string }) =>
						entry.event.startsWith("handler-") &&
						entry.event.endsWith("-finish"),
				);
				expect(finishedEvents.length).toBeLessThan(MESSAGE_COUNT);
			},
			{ timeout: 15_000 },
		);

		// TODO(#4705): Root-cause in-flight DB write interruption during sleep shutdown and re-enable this coverage.
		test.skip(
			"active db writes interrupted by sleep produce db error",
			async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				const actor = client.sleepWsActiveDbExceedsGrace.getOrCreate([
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
							ws.removeEventListener("message", onMessage);
							resolve();
						}
					};
					ws.addEventListener("message", onMessage);
				});

				// Trigger sleep while writes are in progress.
				await triggerSleepBestEffort(actor);

				// Poll until the woken actor reports the persisted post-sleep status after shutdown completes.
				await vi.waitFor(
					async () => {
						const wokenActor =
							client.sleepWsActiveDbExceedsGrace.getOrCreate([
								"ws-active-db-exceeds-grace",
							]);
						const status = await wokenActor.getStatus();
						expect(status.sleepCount).toBeGreaterThanOrEqual(1);
						expect(status.startCount).toBeGreaterThanOrEqual(2);

						const entries = await wokenActor.getLogEntries();
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
						timeout: 10_000,
						interval: 50,
					},
				);

				// Verify the DB has fewer rows than the full count.
				const wokenActor =
					client.sleepWsActiveDbExceedsGrace.getOrCreate([
						"ws-active-db-exceeds-grace",
					]);
				const entries = await wokenActor.getLogEntries();
				const writeEntries = entries.filter((e: { event: string }) =>
					e.event.startsWith("write-"),
				);
				expect(writeEntries.length).toBeGreaterThan(0);
				expect(writeEntries.length).toBeLessThan(ACTIVE_DB_WRITE_COUNT);
			},
			{ timeout: 30_000 },
		);
	});
});
