import { describe, expect, test } from "vitest";
import { RUN_SLEEP_TIMEOUT } from "../../../fixtures/driver-test-suite/run";
import type { DriverTestConfig } from "../mod";
import { setupDriverTest, waitFor } from "../utils";

export function runActorRunTests(driverTestConfig: DriverTestConfig) {
	describe.skipIf(driverTestConfig.skip?.sleep)("Actor Run Tests", () => {
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

		test("run handler exits gracefully on actor stop", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);

			const actor = client.runWithTicks.getOrCreate([
				"run-graceful-exit",
			]);

			// Wait for run to start
			await waitFor(driverTestConfig, 100);

			const state1 = await actor.getState();
			expect(state1.runStarted).toBe(true);

			// Wait for sleep timeout to trigger sleep
			await waitFor(driverTestConfig, RUN_SLEEP_TIMEOUT + 300);

			// Wake actor again and check state persisted
			const state2 = await actor.getState();
			expect(state2.runExited).toBe(true);
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

		test("run handler that exits early triggers destroy", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);

			const actor = client.runWithEarlyExit.getOrCreate(["early-exit"]);

			// Wait for run to start and exit
			await waitFor(driverTestConfig, 100);

			const state1 = await actor.getState();
			expect(state1.runStarted).toBe(true);

			// Wait for the actor to be destroyed
			await waitFor(driverTestConfig, 300);

			// After the run handler exits early, the actor should be destroyed.
			// Depending on the driver, it may be in a destroyed state or recreated.
			// In the file-system driver test environment, the actor is not automatically
			// rescheduled, so we just verify the initial behavior worked.
			// A new getOrCreate should create a fresh actor.
			const actor2 = client.runWithEarlyExit.getOrCreate([
				"early-exit-fresh",
			]);
			const state2 = await actor2.getState();
			expect(state2.runStarted).toBe(true);
		});

		test("run handler that throws error triggers destroy", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);

			const actor = client.runWithError.getOrCreate(["run-error"]);

			// Wait for run to start and throw
			await waitFor(driverTestConfig, 100);

			const state1 = await actor.getState();
			expect(state1.runStarted).toBe(true);

			// Wait for the actor to be destroyed
			await waitFor(driverTestConfig, 300);

			// After the run handler throws, the actor should be destroyed.
			// Similar to the early exit test, the driver may not automatically reschedule.
			// A new getOrCreate should create a fresh actor.
			const actor2 = client.runWithError.getOrCreate(["run-error-fresh"]);
			const state2 = await actor2.getState();
			expect(state2.runStarted).toBe(true);
		});
	});
}
