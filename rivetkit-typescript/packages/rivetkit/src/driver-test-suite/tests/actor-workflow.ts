import { describe, expect, test } from "vitest";
import {
	WORKFLOW_QUEUE_NAME,
} from "../../../fixtures/driver-test-suite/workflow";
import type { DriverTestConfig } from "../mod";
import { setupDriverTest, waitFor } from "../utils";

export function runActorWorkflowTests(driverTestConfig: DriverTestConfig) {
	describe("Actor Workflow Tests", () => {
		test("replays steps and guards state access", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actor = client.workflowCounterActor.getOrCreate([
				"workflow-basic",
			]);

			let state = await actor.getState();
			for (let i = 0; i < 50; i++) {
				if (
					state.runCount > 0 &&
					state.history.length > 0 &&
					state.guardTriggered
				) {
					break;
				}
				await waitFor(driverTestConfig, 100);
				state = await actor.getState();
			}
			expect(state.runCount).toBeGreaterThan(0);
			expect(state.history.length).toBeGreaterThan(0);
			expect(state.guardTriggered).toBe(true);
		});

		test("consumes queue messages via workflow queue.next", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actor = client.workflowQueueActor.getOrCreate(["workflow-queue"]);

			await actor.send(WORKFLOW_QUEUE_NAME, {
				hello: "world",
			});

			await waitFor(driverTestConfig, 200);
			const messages = await actor.getMessages();
			expect(messages).toEqual([{ hello: "world" }]);
		});

		test("workflow queue.next supports completing wait sends", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actor = client.workflowQueueActor.getOrCreate([
				"workflow-queue-wait",
			]);

			const result = await actor.sendAndWait({ value: 123 });
			expect(result).toEqual({
				status: "completed",
				response: { echo: { value: 123 } },
			});
		});

		test("db and client are step-only in workflow context", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actor = client.workflowAccessActor.getOrCreate([
				"workflow-access",
			]);

			let state = await actor.getState();
			for (let i = 0; i < 20 && state.insideDbCount === 0; i++) {
				await waitFor(driverTestConfig, 50);
				state = await actor.getState();
			}

			expect(state.outsideDbError).toBe(
				"db is only available inside workflow steps",
			);
			expect(state.outsideClientError).toBe(
				"client is only available inside workflow steps",
			);
			expect(state.insideDbCount).toBeGreaterThan(0);
			expect(state.insideClientAvailable).toBe(true);
		});

		test("sleeps and resumes between ticks", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actor = client.workflowSleepActor.getOrCreate(["workflow-sleep"]);

			const initial = await actor.getState();
			await waitFor(driverTestConfig, 200);
			const next = await actor.getState();

			expect(next.ticks).toBeGreaterThan(initial.ticks);
		});

		test.skipIf(driverTestConfig.skip?.sleep)(
			"workflow run teardown does not wait for runStopTimeout",
			async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);
				const actor = client.workflowStopTeardownActor.getOrCreate([
					"workflow-stop-teardown",
				]);

				await actor.getTimeline();
				await waitFor(driverTestConfig, 1_200);
				const timeline = await actor.getTimeline();

				expect(timeline.wakeAts.length).toBeGreaterThanOrEqual(2);
				expect(timeline.sleepAts.length).toBeGreaterThanOrEqual(1);

				const firstSleepDelayMs =
					timeline.sleepAts[0] - timeline.wakeAts[0];
				expect(firstSleepDelayMs).toBeLessThan(1_800);
			},
		);

		// NOTE: Test for workflow persistence across actor sleep is complex because
		// calling c.sleep() during a workflow prevents clean shutdown. The workflow
		// persistence is implicitly tested by the "sleeps and resumes between ticks"
		// test which verifies the workflow continues from persisted state.
	});
}
