import { describe, expect, test, vi } from "vitest";
import { RUN_SLEEP_TIMEOUT } from "../../fixtures/driver-test-suite/run";
import { describeDriverMatrix } from "./shared-matrix";
import { setupDriverTest, waitFor } from "./shared-utils";

const RUN_HANDLER_TIMEOUT_MS = 60_000;

describeDriverMatrix("Actor Run", (driverTestConfig) => {
	const describeRunTests = driverTestConfig.skip?.sleep
		? describe.skip
		: describe;

	describeRunTests("Actor Run Tests", () => {
		test("run handler starts after actor startup", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);

			const actor = client.runWithTicks.getOrCreate(["run-starts"]);

			// Wait a bit for run handler to start
			await waitFor(driverTestConfig, 100);

			const state = await actor.getState();
			expect(state.runStarted).toBe(true);
			expect(state.tickCount).toBeGreaterThan(0);
		});

		test("run handler ticks continuously", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);

			const actor = client.runWithTicks.getOrCreate(["run-ticks"]);

			// Wait for some ticks
			await waitFor(driverTestConfig, 200);

			const state1 = await actor.getState();
			expect(state1.tickCount).toBeGreaterThan(0);

			const count1 = state1.tickCount;

			// Wait more and check tick count increased
			await waitFor(driverTestConfig, 200);

			const state2 = await actor.getState();
			expect(state2.tickCount).toBeGreaterThan(count1);
		});

		test("active run handler keeps actor awake past sleep timeout", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);

			const actor = client.runWithTicks.getOrCreate(["run-stays-awake"]);

			// Wait for run to start
			await waitFor(driverTestConfig, 100);

			const state1 = await actor.getState();
			expect(state1.runStarted).toBe(true);
			const tickCount1 = state1.tickCount;

			// Active run loops should keep the actor awake.
			await waitFor(driverTestConfig, RUN_SLEEP_TIMEOUT + 300);

			const state2 = await actor.getState();
			expect(state2.runStarted).toBe(true);
			expect(state2.runExited).toBe(false);
			expect(state2.tickCount).toBeGreaterThan(tickCount1);
		});

		test("actor without run handler works normally", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);

			const actor = client.runWithoutHandler.getOrCreate([
				"no-run-handler",
			]);

			const state = await actor.getState();
			expect(state.wakeCount).toBe(1);

			// Wait for sleep and wake again
			await waitFor(driverTestConfig, RUN_SLEEP_TIMEOUT + 300);

			const state2 = await actor.getState();
			expect(state2.wakeCount).toBe(2);
		});

		test("run handler can consume from queue", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);

			const actor = client.runWithQueueConsumer.getOrCreate([
				"queue-consumer",
			]);

			// Wait for run handler to start
			await waitFor(driverTestConfig, 100);

			// Send some messages to the queue
			await actor.sendMessage({ type: "test", value: 1 });
			await actor.sendMessage({ type: "test", value: 2 });
			await actor.sendMessage({ type: "test", value: 3 });

			// Wait for messages to be consumed
			await waitFor(driverTestConfig, 1200);

			const state = await actor.getState();
			expect(state.runStarted).toBe(true);
			expect(state.messagesReceived.length).toBe(3);
			expect(state.messagesReceived[0].body).toEqual({
				type: "test",
				value: 1,
			});
			expect(state.messagesReceived[1].body).toEqual({
				type: "test",
				value: 2,
			});
			expect(state.messagesReceived[2].body).toEqual({
				type: "test",
				value: 3,
			});
		});

		test(
			"queue-waiting run handler can sleep and resume",
			async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				const actor = client.runWithQueueConsumer.getOrCreate([
					"queue-consumer-sleep",
				]);

				await waitFor(driverTestConfig, 100);
				const state1 = await actor.getState();
				expect(state1.runStarted).toBe(true);

				await waitFor(driverTestConfig, RUN_SLEEP_TIMEOUT + 500);
				const state2 = await actor.getState();

				expect(state2.wakeCount).toBeGreaterThan(state1.wakeCount);
			},
			RUN_HANDLER_TIMEOUT_MS,
		);

		test(
			"run handler that exits early sleeps instead of destroying",
			async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				const observer = client.lifecycleObserver.getOrCreate([
					"run-with-early-exit",
				]);
				await observer.clearEvents();

				const actor = client.runWithEarlyExit.getOrCreate([
					`early-exit-${Date.now()}`,
				]);
				const actorId = await actor.resolve();

				// Wait for run to start and exit
				await waitFor(driverTestConfig, 100);

				const state1 = await actor.getState();
				expect(state1.runStarted).toBe(true);

				if (!driverTestConfig.skip?.sleep) {
					// Poll because the sleep hook is emitted from the actor runtime after idle detection.
					await vi.waitFor(
						async () => {
							const events = await observer.getEvents();
							expect(
								events.filter(
									(event) =>
										event.actorKey === actorId &&
										event.event === "sleep",
								),
							).toHaveLength(1);
						},
						{ timeout: RUN_SLEEP_TIMEOUT + 5_000 },
					);
				} else {
					await waitFor(driverTestConfig, RUN_SLEEP_TIMEOUT + 400);
				}

				const state2 = await actor.getState();
				expect(state2.runStarted).toBe(true);
				expect(state2.destroyCalled).toBe(false);

				if (driverTestConfig.skip?.sleep) {
					expect(state2.sleepCount).toBe(0);
					expect(state2.wakeCount).toBe(1);
				} else {
					expect(state2.sleepCount).toBeGreaterThan(0);
					expect(state2.wakeCount).toBeGreaterThan(1);
				}
			},
			RUN_HANDLER_TIMEOUT_MS,
		);

		test(
			"run handler that throws error sleeps instead of destroying",
			async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);

				const actor = client.runWithError.getOrCreate(["run-error"]);

				// Wait for run to start and throw
				await waitFor(driverTestConfig, 100);

				const state1 = await actor.getState();
				expect(state1.runStarted).toBe(true);

				// Wait for the run handler to throw and the normal idle sleep timeout.
				await waitFor(driverTestConfig, RUN_SLEEP_TIMEOUT + 400);

				const state2 = await actor.getState();
				expect(state2.runStarted).toBe(true);
				expect(state2.destroyCalled).toBe(false);

				if (driverTestConfig.skip?.sleep) {
					expect(state2.sleepCount).toBe(0);
					expect(state2.wakeCount).toBe(1);
				} else {
					expect(state2.sleepCount).toBeGreaterThan(0);
					expect(state2.wakeCount).toBeGreaterThan(1);
				}
			},
			RUN_HANDLER_TIMEOUT_MS,
		);
	});
});
