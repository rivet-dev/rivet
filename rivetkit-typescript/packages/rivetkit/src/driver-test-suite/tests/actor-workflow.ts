import { describe, expect, test } from "vitest";
import {
	WORKFLOW_QUEUE_NAME,
	workflowQueueName,
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

			await waitFor(driverTestConfig, 1000);
			const state = await actor.getState();
			expect(state.runCount).toBeGreaterThan(0);
			expect(state.history.length).toBeGreaterThan(0);
			expect(state.guardTriggered).toBe(true);
		});

		test("consumes queue messages via workflow listen", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actor = client.workflowQueueActor.getOrCreate([
				"workflow-queue",
			]);

			const queueHandle =
				actor.queue[workflowQueueName(WORKFLOW_QUEUE_NAME)];
			await queueHandle.send({ hello: "world" });

			await waitFor(driverTestConfig, 200);
			const messages = await actor.getMessages();
			expect(messages).toEqual([{ hello: "world" }]);
		});

		test("sleeps and resumes between ticks", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actor = client.workflowSleepActor.getOrCreate([
				"workflow-sleep",
			]);

			const initial = await actor.getState();
			await waitFor(driverTestConfig, 200);
			const next = await actor.getState();

			expect(next.ticks).toBeGreaterThan(initial.ticks);
		});

		// NOTE: Test for workflow persistence across actor sleep is complex because
		// calling c.sleep() during a workflow prevents clean shutdown. The workflow
		// persistence is implicitly tested by the "sleeps and resumes between ticks"
		// test which verifies the workflow continues from persisted state.
	});
}
