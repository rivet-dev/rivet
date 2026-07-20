// @ts-nocheck

import { describe, expect, test } from "vitest";
import type { ActorError } from "@/client/mod";
import { MANY_QUEUE_NAMES } from "../../fixtures/driver-test-suite/queue";
import { describeDriverMatrix } from "./shared-matrix";
import { setupDriverTest, waitFor } from "./shared-utils";

const QUEUE_STRESS_TEST_TIMEOUT_MS = 120_000;
const QUEUE_STATUS_POLL_ATTEMPTS = 300;
const QUEUE_SEND_BATCH_SIZE = 16;

describeDriverMatrix("Actor Queue", (driverTestConfig) => {
	describe("Actor Queue Tests", () => {
		async function expectManyQueueChildToDrain(
			handle: Awaited<
				ReturnType<typeof setupDriverTest>
			>["client"]["manyQueueChildActor"],
			key: string,
		) {
			const child = handle.getOrCreate([key]);
			const conn = child.connect();
			const messageCount = MANY_QUEUE_NAMES.length * 4;

			try {
				expect(await conn.ping()).toEqual(
					expect.objectContaining({
						pong: true,
					}),
				);

				for (
					let offset = 0;
					offset < messageCount;
					offset += QUEUE_SEND_BATCH_SIZE
				) {
					await Promise.all(
						Array.from(
							{
								length: Math.min(
									QUEUE_SEND_BATCH_SIZE,
									messageCount - offset,
								),
							},
							(_, batchIndex) => {
								const index = offset + batchIndex;
								return child.send(
									MANY_QUEUE_NAMES[index % MANY_QUEUE_NAMES.length],
									{ index },
								);
							},
						),
					);
				}

				let snapshot = await child.getSnapshot();
				for (
					let i = 0;
					i < QUEUE_STATUS_POLL_ATTEMPTS &&
					snapshot.processed.length < messageCount;
					i++
				) {
					await waitFor(driverTestConfig, 100);
					snapshot = await child.getSnapshot();
				}

				expect(snapshot.started).toBe(true);
				expect(snapshot.processed).toHaveLength(messageCount);
				expect(new Set(snapshot.processed)).toEqual(
					new Set(MANY_QUEUE_NAMES),
				);

				const receipt = await child.send(MANY_QUEUE_NAMES[0], {
					index: messageCount,
				});
				let status = await receipt.status();
				for (
					let i = 0;
					i < QUEUE_STATUS_POLL_ATTEMPTS && status.state !== "succeeded";
					i++
				) {
					await waitFor(driverTestConfig, 100);
					status = await receipt.status();
				}
				expect(status).toMatchObject({ state: "succeeded", attempts: 1 });
			} finally {
				await conn.dispose().catch(() => undefined);
			}
		}

		test("client can send to actor queue", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const handle = client.queueActor.getOrCreate(["client-send"]);

			await handle.send("greeting", { hello: "world" });

			const message = await handle.receiveOne("greeting");
			expect(message).toEqual({
				name: "greeting",
				body: { hello: "world" },
			});
		});

		test("actor can send to its own queue", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const handle = client.queueActor.getOrCreate(["self-send"]);

			await handle.sendToSelf("self", { value: 42 });

			const message = await handle.receiveOne("self");
			expect(message).toEqual({ name: "self", body: { value: 42 } });
		});

		test("nextBatch supports name arrays and counts", { timeout: QUEUE_STRESS_TEST_TIMEOUT_MS }, async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const handle = client.queueActor.getOrCreate(["receive-array"]);

			await handle.send("a", 1);
			await handle.send("b", 2);
			await handle.send("c", 3);

			const messages = await handle.receiveMany(["a", "b"], { count: 2 });
			expect(messages).toEqual([
				{ name: "a", body: 1 },
				{ name: "b", body: 2 },
			]);
		});

		test("nextBatch supports request objects", { timeout: QUEUE_STRESS_TEST_TIMEOUT_MS }, async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const handle = client.queueActor.getOrCreate(["receive-request"]);

			await handle.send("one", "first");
			await handle.send("two", "second");

			const messages = await handle.receiveRequest({
				names: ["one", "two"],
				count: 2,
			});
			expect(messages).toEqual([
				{ name: "one", body: "first" },
				{ name: "two", body: "second" },
			]);
		});

		test("nextBatch defaults to all names when names is omitted", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const handle = client.queueActor.getOrCreate([
				"receive-request-all",
			]);

			await handle.send("one", "first");
			await handle.send("two", "second");

			const messages = await handle.receiveRequest({ count: 2 });
			expect(messages).toEqual([
				{ name: "one", body: "first" },
				{ name: "two", body: "second" },
			]);
		});

		test("next timeout returns empty array", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const handle = client.queueActor.getOrCreate(["receive-timeout"]);

			const promise = handle.receiveMany(["missing"], { timeout: 50 });
			await waitFor(driverTestConfig, 60);
			const messages = await promise;
			expect(messages).toEqual([]);
		});

		test("tryNextBatch does not wait and returns empty array", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const handle = client.queueActor.getOrCreate(["try-next-empty"]);

			const messages = await handle.tryReceiveMany({
				names: ["missing"],
				count: 1,
			});
			expect(messages).toEqual([]);
		});

		test("abort throws ActorAborted", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const handle = client.queueActor.getOrCreate(["abort-test"]);

			try {
				await handle.waitForAbort();
				expect.fail("expected ActorAborted error");
			} catch (error) {
				expect((error as ActorError).group).toBe("actor");
				expect((error as ActorError).code).toBe("aborted");
			}
		});

		test("next supports signal abort", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const handle = client.queueActor.getOrCreate(["signal-abort-next"]);

			const result = await handle.waitForSignalAbort();
			expect(result).toEqual({
				group: "actor",
				code: "aborted",
			});
		});

		test("next supports actor abort when signal is provided", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const handle = client.queueActor.getOrCreate([
				"actor-abort-with-signal-next",
			]);

			const result = await handle.waitForActorAbortWithSignal();
			expect(result).toEqual({
				group: "actor",
				code: "aborted",
			});
		});

		test("iter supports signal abort", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const handle = client.queueActor.getOrCreate(["signal-abort-iter"]);

			const result = await handle.iterWithSignalAbort();
			expect(result).toEqual({ ok: true });
		});

		test("enforces queue size limit", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const key = `size-limit-${Date.now()}-${Math.random().toString(16).slice(2)}`;
			const handle = client.queueLimitedActor.getOrCreate([key]);

			await handle.send("message", 1);

			await waitFor(driverTestConfig, 10);

			try {
				await handle.send("message", 2);
				expect.fail("expected queue full error");
			} catch (error) {
				expect(error).toBeInstanceOf(Error);
				expect((error as Error).message).toContain(
					"Queue is full. Limit is",
				);
			}
		});

		test("enforces message size limit", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const handle = client.queueLimitedActor.getOrCreate([
				"message-limit",
			]);
			const largePayload = "a".repeat(200);

			try {
				await handle.send("oversize", largePayload);
				expect.fail("expected message_too_large error");
			} catch (error) {
				expect((error as ActorError).group).toBe("queue");
				expect((error as ActorError).code).toBe("message_too_large");
			}
		});

		test("send returns a receipt whose handler status becomes succeeded", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const handle = client.queueActor.getOrCreate(["handler-receipt"]);
			const receipt = await handle.send("handled", { value: 123 });
			expect(receipt.id).toEqual(expect.any(String));
			expect(receipt.deduplicated).toBe(false);

			let status = await receipt.status();
			for (
				let i = 0;
				i < QUEUE_STATUS_POLL_ATTEMPTS && status.state !== "succeeded";
				i++
			) {
				await waitFor(driverTestConfig, 100);
				status = await handle.receipt(receipt.id).status();
			}
			expect(status).toMatchObject({ state: "succeeded", attempts: 1 });
			expect(await handle.getQueueState()).toMatchObject({ handled: 123 });
			await expect(handle.receiveOne("handled", { timeout: 0 })).rejects.toMatchObject({
				group: "queue",
				code: "automatic_consumer",
			});
		});

		test("dedupeKey returns the original receipt", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const handle = client.queueActor.getOrCreate(["dedupe"]);
			const first = await handle.send(
				"handled",
				{ value: 1 },
				{ dedupeKey: "same" },
			);
			const second = await handle.send(
				"handled",
				{ value: 999 },
				{ dedupeKey: "same" },
			);
			expect(second.id).toBe(first.id);
			expect(second.deduplicated).toBe(true);
		});

		test(
			"drains many-queue child actors created from actions while connected",
			{ timeout: QUEUE_STRESS_TEST_TIMEOUT_MS },
			async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);
				const parent = client.manyQueueActionParentActor.getOrCreate([
					"many-action-parent",
				]);

				expect(await parent.spawnChild("many-action-child")).toEqual({
					key: "many-action-child",
				});

				await expectManyQueueChildToDrain(
					client.manyQueueChildActor,
					"many-action-child",
				);
			},
		);

		test(
			"drains many-queue child actors created from run handlers while connected",
			{ timeout: QUEUE_STRESS_TEST_TIMEOUT_MS },
			async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);
				const parent = client.manyQueueRunParentActor.getOrCreate([
					"many-run-parent",
				]);

				expect(await parent.queueSpawn("many-run-child")).toEqual({
					queued: true,
				});

				let spawned = await parent.getSpawned();
				for (
					let i = 0;
					i < QUEUE_STATUS_POLL_ATTEMPTS &&
					!spawned.includes("many-run-child");
					i++
				) {
					await waitFor(driverTestConfig, 100);
					spawned = await parent.getSpawned();
				}
				expect(spawned).toContain("many-run-child");

				await expectManyQueueChildToDrain(
					client.manyQueueChildActor,
					"many-run-child",
				);
			},
		);

		test("raw receive is consumed before it is returned", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const handle = client.queueActor.getOrCreate(["raw-consumed"]);
			const receipt = await handle.send("tasks", { value: 789 });
			expect(await handle.receiveOne("tasks")).toEqual({
				name: "tasks",
				body: { value: 789 },
			});
			expect(await receipt.status()).toMatchObject({ state: "consumed" });
			expect(await handle.receiveOne("tasks", { timeout: 10 })).toBeNull();
		});

		test("handler throws are retried until success", { timeout: QUEUE_STRESS_TEST_TIMEOUT_MS }, async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const handle = client.queueActor.getOrCreate(["handler-retry"]);
			const receipt = await handle.send("retrying", { value: 1 });
			let status = await receipt.status();
			for (
				let i = 0;
				i < QUEUE_STATUS_POLL_ATTEMPTS && status.state !== "succeeded";
				i++
			) {
				await waitFor(driverTestConfig, 100);
				status = await receipt.status();
			}
			expect(status).toMatchObject({ state: "succeeded", attempts: 3 });
		});

		test("handler timeout aborts without waiting for the promise to settle", { timeout: QUEUE_STRESS_TEST_TIMEOUT_MS }, async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const handle = client.queueActor.getOrCreate(["handler-timeout"]);
			const receipt = await handle.send("timedHandler", { value: 1 });
			let status = await receipt.status();
			for (
				let i = 0;
				i < QUEUE_STATUS_POLL_ATTEMPTS && status.state !== "succeeded";
				i++
			) {
				await waitFor(driverTestConfig, 100);
				status = await receipt.status();
			}
			expect(status).toMatchObject({ state: "succeeded", attempts: 2 });
			expect(await handle.getQueueState()).toMatchObject({
				timeoutAttempts: 2,
				timeoutAborted: true,
			});
		});

		test("exhausted handler attempts become dead-lettered", { timeout: QUEUE_STRESS_TEST_TIMEOUT_MS }, async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const handle = client.queueActor.getOrCreate(["handler-dead"]);
			const receipt = await handle.send("dead", { value: 1 });
			let status = await receipt.status();
			for (
				let i = 0;
				i < QUEUE_STATUS_POLL_ATTEMPTS && status.state !== "deadLettered";
				i++
			) {
				await waitFor(driverTestConfig, 100);
				status = await receipt.status();
			}
			expect(status).toMatchObject({ state: "deadLettered", attempts: 2 });
		});

		test("iter can consume queued messages", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const handle = client.queueActor.getOrCreate(["iter-consume"]);

			await handle.send("one", "first");
			const message = await handle.receiveWithIterator("one");
			expect(message).toEqual({ name: "one", body: "first" });
		});

		test("queue async iterator can consume queued messages", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const handle = client.queueActor.getOrCreate([
				"async-iter-consume",
			]);

			await handle.send("two", "second");
			const message = await handle.receiveWithAsyncIterator();
			expect(message).toEqual({ name: "two", body: "second" });
		});
	});
});
