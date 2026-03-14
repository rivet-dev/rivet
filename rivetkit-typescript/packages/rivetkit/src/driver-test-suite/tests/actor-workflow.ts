import { describe, expect, test, vi } from "vitest";
import { WORKFLOW_QUEUE_NAME } from "../../../fixtures/driver-test-suite/workflow";
import type { ActorError } from "@/client/mod";
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
			const actor = client.workflowQueueActor.getOrCreate([
				"workflow-queue",
			]);

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
			const actor = client.workflowSleepActor.getOrCreate([
				"workflow-sleep",
			]);

			const initial = await actor.getState();
			await waitFor(driverTestConfig, 200);
			const next = await actor.getState();

			expect(next.ticks).toBeGreaterThan(initial.ticks);
		});

		test("workflow onError reports retry metadata", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actor = client.workflowErrorHookActor.getOrCreate([
				"workflow-error-hook",
			]);

			let state = await actor.getErrorState();
			for (
				let i = 0;
				i < 80 && (state.attempts < 2 || state.events.length === 0);
				i++
			) {
				await waitFor(driverTestConfig, 50);
				state = await actor.getErrorState();
			}

			expect(state.attempts).toBe(2);
			expect(state.events).toHaveLength(1);
			expect(state.events[0]).toEqual(
				expect.objectContaining({
					step: expect.objectContaining({
						stepName: "flaky",
						attempt: 1,
						willRetry: true,
						retryDelay: 1,
						error: expect.objectContaining({
							name: "Error",
							message: "workflow hook failed",
						}),
					}),
				}),
			);
		});

		test("workflow onError can update actor state", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actor = client.workflowErrorHookEffectsActor.getOrCreate([
				"workflow-error-state",
			]);

			await actor.startWorkflow();

			let state = await actor.getErrorState();
			for (
				let i = 0;
				i < 80 &&
				(state.attempts < 2 ||
					state.lastError === null ||
					state.errorCount === 0);
				i++
			) {
				await waitFor(driverTestConfig, 50);
				state = await actor.getErrorState();
			}

			expect(state.attempts).toBe(2);
			expect(state.errorCount).toBe(1);
			expect(state.lastError).toEqual(
				expect.objectContaining({
					step: expect.objectContaining({
						stepName: "flaky",
						attempt: 1,
						willRetry: true,
						retryDelay: 1,
						error: expect.objectContaining({
							name: "Error",
							message: "workflow hook failed",
						}),
					}),
				}),
			);
		});

		test("workflow onError can broadcast actor events", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actor = client.workflowErrorHookEffectsActor
				.getOrCreate(["workflow-error-broadcast"])
				.connect();

			try {
				const eventPromise = new Promise((resolve) => {
					actor.once("workflowError", resolve);
				});

				await actor.startWorkflow();

				const event = await eventPromise;
				expect(event).toEqual(
					expect.objectContaining({
						step: expect.objectContaining({
							stepName: "flaky",
							attempt: 1,
							willRetry: true,
							retryDelay: 1,
							error: expect.objectContaining({
								name: "Error",
								message: "workflow hook failed",
							}),
						}),
					}),
				);
			} finally {
				await actor.dispose();
			}
		});

		test("workflow onError can enqueue actor messages", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actor = client.workflowErrorHookEffectsActor.getOrCreate([
				"workflow-error-queue",
			]);

			await actor.startWorkflow();

			const queuedError = await actor.receiveQueuedError();
			expect(queuedError).toEqual(
				expect.objectContaining({
					step: expect.objectContaining({
						stepName: "flaky",
						attempt: 1,
						willRetry: true,
						retryDelay: 1,
						error: expect.objectContaining({
							name: "Error",
							message: "workflow hook failed",
						}),
					}),
				}),
			);
		});
		
		test("destroyOnCompletion destroys the actor", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);

			const actorKey = "workflow-destroy-on-completion";
			const observer = client.destroyObserver.getOrCreate(["observer"]);
			await observer.reset();

			const actor = client.workflowDestroyOnCompletionActor.getOrCreate([
				actorKey,
			]);
			const actorId = await actor.resolve();

			await vi.waitFor(async () => {
				const wasDestroyed = await observer.wasDestroyed(actorKey);
				expect(wasDestroyed, "actor onDestroy not called").toBeTruthy();
			});

			await vi.waitFor(async () => {
				let actorRunning = false;
				try {
					await client.workflowDestroyOnCompletionActor
						.getForId(actorId)
						.getRunCount();
					actorRunning = true;
				} catch (err) {
					expect((err as ActorError).group).toBe("actor");
					expect((err as ActorError).code).toBe("not_found");
				}

				expect(actorRunning, "actor still running").toBeFalsy();
			});
		});

		test.skipIf(driverTestConfig.skip?.sleep)(
			"completed workflows sleep instead of destroying the actor",
			async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);
				const actor = client.workflowCompleteActor.getOrCreate([
					"workflow-complete",
				]);

				let state = await actor.getState();
				for (let i = 0; i < 10 && state.sleepCount === 0; i++) {
					await waitFor(driverTestConfig, 100);
					state = await actor.getState();
				}
				expect(state.runCount).toBeGreaterThan(0);
				expect(state.sleepCount).toBeGreaterThan(0);
				expect(state.startCount).toBeGreaterThan(1);
			},
		);

		test.skipIf(driverTestConfig.skip?.sleep)(
			"failed workflow steps sleep instead of surfacing as run errors",
			async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);
				const actor = client.workflowFailedStepActor.getOrCreate([
					"workflow-failed-step",
				]);

				let state = await actor.getState();
				for (let i = 0; i < 10 && state.sleepCount === 0; i++) {
					await waitFor(driverTestConfig, 100);
					state = await actor.getState();
				}
				expect(state.runCount).toBeGreaterThan(0);
				expect(state.sleepCount).toBeGreaterThan(0);
				expect(state.startCount).toBeGreaterThan(1);
			},
		);

		test.skipIf(driverTestConfig.skip?.sleep)(
			"workflow onError is not reported again after sleep and wake",
			async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);
				const actor = client.workflowErrorHookSleepActor.getOrCreate([
					"workflow-error-hook-sleep",
				]);

				let state = await actor.getErrorState();
				for (
					let i = 0;
					i < 80 &&
					(state.attempts < 2 || state.events.length === 0);
					i++
				) {
					await waitFor(driverTestConfig, 50);
					state = await actor.getErrorState();
				}

				expect(state.attempts).toBe(2);
				expect(state.events).toHaveLength(1);
				expect(state.wakeCount).toBe(1);

				await actor.triggerSleep();
				await waitFor(driverTestConfig, 250);

				let resumedState = await actor.getErrorState();
				for (
					let i = 0;
					i < 40 &&
					(resumedState.wakeCount < 2 || resumedState.sleepCount < 1);
					i++
				) {
					await waitFor(driverTestConfig, 50);
					resumedState = await actor.getErrorState();
				}

				expect(resumedState.sleepCount).toBeGreaterThanOrEqual(1);
				expect(resumedState.wakeCount).toBeGreaterThanOrEqual(2);
				expect(resumedState.attempts).toBe(2);
				expect(resumedState.events).toHaveLength(1);
			},
		);

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
