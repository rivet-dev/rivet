import { describe, expect, test } from "vitest";
import type { ActorError } from "@/client/mod";
import type { DriverTestConfig } from "../mod";
import { setupDriverTest, waitFor } from "../utils";

export function runActorQueueTests(driverTestConfig: DriverTestConfig) {
	describe("Actor Queue Tests", () => {
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

		test("nextBatch supports name arrays and counts", async (c) => {
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

		test("nextBatch supports request objects", async (c) => {
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
				if (driverTestConfig.clientType !== "http") {
					expect((error as ActorError).group).toBe("queue");
					expect((error as ActorError).code).toBe("full");
				}
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

			test("wait send returns completion response", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);
				const handle = client.queueActor.getOrCreate(["wait-complete"]);
				const waitTimeout = driverTestConfig.useRealTimers ? 5_000 : 1_000;

				const actionPromise = handle.receiveAndComplete("tasks");
				const result = await handle.send("tasks", 
					{ value: 123 },
					{ wait: true, timeout: waitTimeout },
				);

			await actionPromise;
			expect(result).toEqual({
				status: "completed",
				response: { echo: { value: 123 } },
			});
		});

		test("wait send times out", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const handle = client.queueActor.getOrCreate(["wait-timeout"]);

			const resultPromise = handle.send("timeout", 
				{ value: 456 },
				{ wait: true, timeout: 50 },
			);

			await waitFor(driverTestConfig, 60);
			const result = await resultPromise;

			expect(result.status).toBe("timedOut");
		});

		test("manual receive retries message when not completed", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const handle = client.queueActor.getOrCreate([
				"manual-retry-uncompleted",
			]);

			await handle.send("tasks", { value: 789 });
			const first = await handle.receiveWithoutComplete("tasks");
			expect(first).toEqual({ name: "tasks", body: { value: 789 } });

			const retried = await handle.receiveOne("tasks", { timeout: 1_000 });
			expect(retried).toEqual({ name: "tasks", body: { value: 789 } });
		});

		test("next throws when previous manual message is not completed", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const handle = client.queueActor.getOrCreate([
				"manual-next-requires-complete",
			]);

			await handle.send("tasks", { value: 111 });
			const result = await handle.receiveManualThenNextWithoutComplete(
				"tasks",
			);
			expect(result).toEqual({
				group: "queue",
				code: "previous_message_not_completed",
			});
		});

		test("manual receive includes complete even without completion schema", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const handle = client.queueActor.getOrCreate([
				"complete-not-allowed",
			]);

			await handle.send("nowait", { value: "test" });
			const result = await handle.receiveWithoutCompleteMethod("nowait");

			expect(result).toEqual({
				hasComplete: true,
			});
		});

		test("manual receive retries queues without completion schema until completed", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const handle = client.queueActor.getOrCreate([
				"complete-not-allowed-consume",
			]);

			await handle.send("nowait", { value: "test" });
			const result = await handle.receiveWithoutCompleteMethod("nowait");
			expect(result).toEqual({ hasComplete: true });

			const next = await handle.receiveOne("nowait", { timeout: 1_000 });
			expect(next).toEqual({ name: "nowait", body: { value: "test" } });
		});

		test("complete throws when called twice", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const handle = client.queueActor.getOrCreate([
				"complete-twice",
			]);

			await handle.send("twice", { value: "test" });
			const result = await handle.receiveAndCompleteTwice("twice");

			expect(result).toEqual({
				group: "queue",
				code: "already_completed",
			});
		});

		test("wait send no longer requires queue completion schema", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const handle = client.queueActor.getOrCreate([
				"missing-completion-schema",
			]);

			const result = await handle.send(
				"nowait",
				{ value: "test" },
				{ wait: true, timeout: 50 },
			);
			expect(result).toEqual({ status: "timedOut" });
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
}
