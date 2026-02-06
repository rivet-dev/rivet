import { describe, expect, test } from "vitest";
import type { DriverTestConfig } from "../mod";
import { setupDriverTest, waitFor } from "../utils";

export function runActorScheduleTests(driverTestConfig: DriverTestConfig) {
	describe.skipIf(driverTestConfig.skip?.schedule)(
		"Actor Schedule Tests",
		() => {
			// See alarm + actor sleeping test in actor-sleep.ts

			describe("Scheduled Alarms", () => {
				test("executes c.schedule.at() with specific timestamp", async (c) => {
					const { client } = await setupDriverTest(
						c,
						driverTestConfig,
					);

					// Create instance
					const scheduled = client.scheduled.getOrCreate();

					// Schedule a task to run using timestamp
					const timestamp = Date.now() + 250;
					await scheduled.scheduleTaskAt(timestamp);

					// Wait for longer than the scheduled time
					await waitFor(driverTestConfig, 500);

					// Verify the scheduled task ran
					const lastRun = await scheduled.getLastRun();
					const scheduledCount = await scheduled.getScheduledCount();

					expect(lastRun).toBeGreaterThan(0);
					expect(scheduledCount).toBe(1);
				});

				test("executes c.schedule.after() with delay", async (c) => {
					const { client } = await setupDriverTest(
						c,
						driverTestConfig,
					);

					// Create instance
					const scheduled = client.scheduled.getOrCreate();

					// Schedule a task to run using delay
					await scheduled.scheduleTaskAfter(250);

					// Wait for longer than the scheduled time
					await waitFor(driverTestConfig, 500);

					// Verify the scheduled task ran
					const lastRun = await scheduled.getLastRun();
					const scheduledCount = await scheduled.getScheduledCount();

					expect(lastRun).toBeGreaterThan(0);
					expect(scheduledCount).toBe(1);
				});

				test("multiple scheduled tasks execute in order", async (c) => {
					const { client } = await setupDriverTest(
						c,
						driverTestConfig,
					);

					// Create instance
					const scheduled = client.scheduled.getOrCreate();

					// Reset history to start fresh
					await scheduled.clearHistory();

					// Schedule multiple tasks with different delays
					await scheduled.scheduleTaskAfterWithId("first", 250);
					await scheduled.scheduleTaskAfterWithId("second", 750);
					await scheduled.scheduleTaskAfterWithId("third", 1250);

					// Wait for first task only
					await waitFor(driverTestConfig, 500);
					const history1 = await scheduled.getTaskHistory();
					expect(history1[0]).toBe("first");

					// Wait for second task
					await waitFor(driverTestConfig, 500);
					const history2 = await scheduled.getTaskHistory();
					expect(history2.slice(0, 2)).toEqual([
						"first",
						"second",
					]);

					// Wait for third task
					await waitFor(driverTestConfig, 500);
					const history3 = await scheduled.getTaskHistory();
					expect(history3).toEqual(["first", "second", "third"]);
				});
			});
		},
	);
}
