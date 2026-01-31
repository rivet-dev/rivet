import { describe, expect, test } from "vitest";
import type { ActorError } from "@/client/mod";
import type { DriverTestConfig } from "../mod";
import { setupDriverTest, waitFor } from "../utils";

export function runActorQueueTests(driverTestConfig: DriverTestConfig) {
	describe("Actor Queue Tests", () => {
		test("client can send to actor queue", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const handle = client.queueActor.getOrCreate(["client-send"]);

			await handle.queue.greeting.send({ hello: "world" });

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

		test("next supports name arrays and counts", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const handle = client.queueActor.getOrCreate(["receive-array"]);

			await handle.queue.a.send(1);
			await handle.queue.b.send(2);
			await handle.queue.c.send(3);

			const messages = await handle.receiveMany(["a", "b"], { count: 2 });
			expect(messages).toEqual([
				{ name: "a", body: 1 },
				{ name: "b", body: 2 },
			]);
		});

		test("next supports request objects", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const handle = client.queueActor.getOrCreate(["receive-request"]);

			await handle.queue.one.send("first");
			await handle.queue.two.send("second");

			const messages = await handle.receiveRequest({
				name: ["one", "two"],
				count: 2,
			});
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

		test("enforces queue size limit", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const key = `size-limit-${Date.now()}-${Math.random().toString(16).slice(2)}`;
			const handle = client.queueLimitedActor.getOrCreate([key]);

			await handle.queue.message.send(1);

			await waitFor(driverTestConfig, 10);

			try {
				await handle.queue.message.send(2);
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
				await handle.queue.oversize.send(largePayload);
				expect.fail("expected message_too_large error");
			} catch (error) {
				expect((error as ActorError).group).toBe("queue");
				expect((error as ActorError).code).toBe("message_too_large");
			}
		});

		test("wait send returns completion response", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const handle = client.queueActor.getOrCreate(["wait-complete"]);

			const actionPromise = handle.receiveAndComplete("tasks");
			const result = await handle.queue.tasks.send(
				{ value: 123 },
				{ wait: true, timeout: 1_000 },
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

			const resultPromise = handle.queue.timeout.send(
				{ value: 456 },
				{ wait: true, timeout: 50 },
			);

			await waitFor(driverTestConfig, 60);
			const result = await resultPromise;

			expect(result.status).toBe("timedOut");
		});

		test("complete throws when wait is false", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const handle = client.queueActor.getOrCreate([
				"complete-not-allowed",
			]);

			await handle.queue.nowait.send({ value: "test" });
			const result = await handle.receiveWithoutWaitComplete("nowait");

			expect(result).toEqual({
				group: "queue",
				code: "complete_not_allowed",
			});
		});

		test("complete throws when called twice", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const handle = client.queueActor.getOrCreate([
				"complete-twice",
			]);

			await handle.queue.twice.send({ value: "test" });
			const result = await handle.receiveAndCompleteTwice("twice");

			expect(result).toEqual({
				group: "queue",
				code: "already_completed",
			});
		});

		test("next throws when message pending", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const handle = client.queueActor.getOrCreate(["pending-next"]);

			await handle.queue.pending.send({ value: "test" });
			const result = await handle.receiveWhilePending("pending");

			expect(result).toEqual({
				group: "queue",
				code: "message_pending",
			});
		});
	});
}
