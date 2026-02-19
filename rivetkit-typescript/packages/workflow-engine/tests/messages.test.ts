import { beforeEach, describe, expect, it } from "vitest";
import {
	buildMessageKey,
	generateId,
	InMemoryDriver,
	runWorkflow,
	serializeMessage,
	type WorkflowContextInterface,
	type WorkflowMessageDriver,
} from "../src/testing.js";

function buildMessagePayload(name: string, data: string, id = generateId()) {
	return {
		id,
		name,
		data,
		sentAt: Date.now(),
	};
}

const modes = ["yield", "live"] as const;

for (const mode of modes) {
	describe(`Workflow Engine Messages (${mode})`, { sequential: true }, () => {
		let driver: InMemoryDriver;

		beforeEach(() => {
			driver = new InMemoryDriver();
			driver.latency = 0;
		});

		it("should wait for messages", async () => {
			const workflow = async (ctx: WorkflowContextInterface) => {
				const [message] = await ctx.queue.next<string>("wait-message", {
					names: ["my-message"],
				});
				if (!message) {
					throw new Error("Expected message");
				}
				return message.body;
			};

			const handle = runWorkflow("wf-1", workflow, undefined, driver, {
				mode,
			});

			if (mode === "yield") {
				const result1 = await handle.result;
				expect(result1.state).toBe("sleeping");
				expect(result1.waitingForMessages).toContain("my-message");
				return;
			}

			await handle.message("my-message", "payload");
			const result = await handle.result;
			expect(result.state).toBe("completed");
			expect(result.output).toBe("payload");
		});

		it("should wait for any message in a name set", async () => {
			const workflow = async (ctx: WorkflowContextInterface) => {
				const [message] = await ctx.queue.next<string>("wait-many", {
					names: ["first", "second"],
				});
				if (!message) {
					throw new Error("Expected message");
				}
				return { name: message.name, body: message.body };
			};

			const handle = runWorkflow("wf-1", workflow, undefined, driver, {
				mode,
			});

			if (mode === "yield") {
				const result1 = await handle.result;
				expect(result1.state).toBe("sleeping");
				expect(result1.waitingForMessages).toEqual(["first", "second"]);

				await handle.message("second", "payload");
				const result2 = await runWorkflow(
					"wf-1",
					workflow,
					undefined,
					driver,
					{ mode },
				).result;
				expect(result2.state).toBe("completed");
				expect(result2.output).toEqual({
					name: "second",
					body: "payload",
				});
				return;
			}

			await handle.message("second", "payload");
			const result = await handle.result;
			expect(result.state).toBe("completed");
			expect(result.output).toEqual({
				name: "second",
				body: "payload",
			});
		});

			it("should consume pending messages", async () => {
			const messageId = generateId();
			await driver.set(
				buildMessageKey(messageId),
				serializeMessage(
					buildMessagePayload("my-message", "hello", messageId),
				),
			);

				const workflow = async (ctx: WorkflowContextInterface) => {
					const [message] = await ctx.queue.next<string>("wait-message", {
						names: ["my-message"],
					});
					if (!message) {
						throw new Error("Expected message");
					}
					return message.body;
				};

			const result = await runWorkflow(
				"wf-1",
				workflow,
				undefined,
				driver,
				{
					mode,
				},
			).result;

			expect(result.state).toBe("completed");
			expect(result.output).toBe("hello");
		});

			it("queue.next should return completable messages", async () => {
			const completions: Array<{ id: string; response?: unknown }> = [];
			const pending = [
				buildMessagePayload("my-message", "hello", "msg-1") as {
					id: string;
					name: string;
					data: unknown;
					sentAt: number;
					complete?: (response?: unknown) => Promise<void>;
				},
			];

			const messageDriver: WorkflowMessageDriver = {
				async loadMessages() {
					return pending.map((message) => ({
						...message,
						complete: async (response?: unknown) => {
							completions.push({ id: message.id, response });
						},
					}));
				},
				async addMessage(message) {
					pending.push(message);
				},
				async deleteMessages(messageIds) {
					const deleted = new Set(messageIds);
					const remaining = pending.filter(
						(message) => !deleted.has(message.id),
					);
					pending.length = 0;
					pending.push(...remaining);
					return messageIds;
				},
			};
			driver.messageDriver = messageDriver;

				const workflow = async (ctx: WorkflowContextInterface) => {
					const [message] = await ctx.queue.next<string>("wait-message", {
						names: ["my-message"],
						completable: true,
					});
					if (!message) {
						throw new Error("Expected message");
					}
					if (!message.complete) {
						throw new Error("Expected completable message");
					}
					await message.complete({ ok: true });
					return message.body;
				};

			const result = await runWorkflow(
				"wf-1",
				workflow,
				undefined,
				driver,
				{
					mode,
				},
			).result;

			expect(result.state).toBe("completed");
			expect(result.output).toBe("hello");
			expect(completions).toEqual([{ id: "msg-1", response: { ok: true } }]);
		});

		it("replay should not block the next completable queue.next", async () => {
			if (mode !== "yield") {
				return;
			}

			await driver.set(
				buildMessageKey("msg-1"),
				serializeMessage(buildMessagePayload("my-message", "one", "msg-1")),
			);
			await driver.set(
				buildMessageKey("msg-2"),
				serializeMessage(buildMessagePayload("my-message", "two", "msg-2")),
			);

			const workflow = async (ctx: WorkflowContextInterface) => {
				const [first] = await ctx.queue.next<string>("wait-first", {
					names: ["my-message"],
					completable: true,
				});
				if (!first || !first.complete) {
					throw new Error("Expected first completable message");
				}
				const completeFirst = first.complete;
				await ctx.step("complete-first", async () => {
					await completeFirst({ ok: "first" });
					return first.body;
				});

				await ctx.sleep("between", 120);

				const [second] = await ctx.queue.next<string>("wait-second", {
					names: ["my-message"],
					completable: true,
				});
				if (!second || !second.complete) {
					throw new Error("Expected second completable message");
				}
				const completeSecond = second.complete;
				await ctx.step("complete-second", async () => {
					await completeSecond({ ok: "second" });
					return second.body;
				});

				return [first.body, second.body] as const;
			};

			const firstRun = await runWorkflow(
				"wf-1",
				workflow,
				undefined,
				driver,
				{ mode },
			).result;
			expect(firstRun.state).toBe("sleeping");

			await new Promise((resolve) => setTimeout(resolve, 140));

			const secondRun = await runWorkflow(
				"wf-1",
				workflow,
				undefined,
				driver,
				{ mode },
			).result;
			expect(secondRun.state).toBe("completed");
			expect(secondRun.output).toEqual(["one", "two"]);
		});

		it("replay should keep blocking if completable message was not completed", async () => {
			if (mode !== "yield") {
				return;
			}

			await driver.set(
				buildMessageKey("msg-1"),
				serializeMessage(
					buildMessagePayload("my-message", "one", "msg-1"),
				),
			);
			await driver.set(
				buildMessageKey("msg-2"),
				serializeMessage(
					buildMessagePayload("my-message", "two", "msg-2"),
				),
			);

			const workflow = async (ctx: WorkflowContextInterface) => {
				const [first] = await ctx.queue.next<string>("wait-first", {
					names: ["my-message"],
					completable: true,
				});
				if (!first || !first.complete) {
					throw new Error("Expected first completable message");
				}

				// Intentionally do not complete the message.
				await ctx.sleep("between", 120);

				await ctx.queue.next<string>("wait-second", {
					names: ["my-message"],
					completable: true,
				});

				return first.body;
			};

			const firstRun = await runWorkflow(
				"wf-1",
				workflow,
				undefined,
				driver,
				{ mode },
			).result;
			expect(firstRun.state).toBe("sleeping");

			await new Promise((resolve) => setTimeout(resolve, 140));

			const secondRunHandle = runWorkflow(
				"wf-1",
				workflow,
				undefined,
				driver,
				{ mode },
			);
			await expect(secondRunHandle.result).rejects.toThrow(
				"Previous completable queue message is not completed.",
			);

			const queued = await driver.messageDriver.loadMessages();
			expect(queued.map((message) => message.id).sort()).toEqual([
				"msg-1",
				"msg-2",
			]);
		});

			it("should collect multiple messages with queue.next count", async () => {
			await driver.set(
				buildMessageKey("1"),
				serializeMessage(buildMessagePayload("batch", "a", "1")),
			);
			await driver.set(
				buildMessageKey("2"),
				serializeMessage(buildMessagePayload("batch", "b", "2")),
			);

				const workflow = async (ctx: WorkflowContextInterface) => {
					const messages = await ctx.queue.next<string>("batch-wait", {
						names: ["batch"],
						count: 2,
					});
					return messages.map((message) => message.body);
				};

			const result = await runWorkflow(
				"wf-1",
				workflow,
				undefined,
				driver,
				{
					mode,
				},
			).result;

			expect(result.state).toBe("completed");
			expect(result.output).toEqual(["a", "b"]);
		});

			it("should time out queue.next", async () => {
				const workflow = async (ctx: WorkflowContextInterface) => {
					const messages = await ctx.queue.next<string>("timeout", {
						names: ["missing"],
						timeout: 50,
					});
					return messages[0]?.body ?? null;
				};

			if (mode === "yield") {
				const result1 = await runWorkflow(
					"wf-1",
					workflow,
					undefined,
					driver,
					{ mode },
				).result;
				expect(result1.state).toBe("sleeping");

				await new Promise((r) => setTimeout(r, 80));

				const result2 = await runWorkflow(
					"wf-1",
					workflow,
					undefined,
					driver,
					{ mode },
				).result;
				expect(result2.state).toBe("completed");
				expect(result2.output).toBeNull();
				return;
			}

			const result = await runWorkflow(
				"wf-1",
				workflow,
				undefined,
				driver,
				{ mode },
			).result;
			expect(result.state).toBe("completed");
			expect(result.output).toBeNull();
		});

			it("should return a message before queue.next timeout", async () => {
			const messageId = generateId();
			await driver.set(
				buildMessageKey(messageId),
				serializeMessage(
					buildMessagePayload("deadline", "data", messageId),
				),
			);

				const workflow = async (ctx: WorkflowContextInterface) => {
					const messages = await ctx.queue.next<string>("deadline", {
						names: ["deadline"],
						timeout: 1000,
					});
					return messages[0]?.body ?? null;
				};

			const result = await runWorkflow(
				"wf-1",
				workflow,
				undefined,
				driver,
				{
					mode,
				},
			).result;

			expect(result.state).toBe("completed");
			expect(result.output).toBe("data");
		});

			it("should wait for queue.next timeout messages", async () => {
				const workflow = async (ctx: WorkflowContextInterface) => {
					const messages = await ctx.queue.next<string>("batch", {
						names: ["batch"],
						count: 2,
						timeout: 5000,
					});
					return messages.map((message) => message.body);
				};

			const handle = runWorkflow("wf-1", workflow, undefined, driver, {
				mode,
			});

			if (mode === "yield") {
				const result1 = await handle.result;
				expect(result1.state).toBe("sleeping");

				await handle.message("batch", "first");

				const result2 = await runWorkflow(
					"wf-1",
					workflow,
					undefined,
					driver,
					{ mode },
				).result;
				expect(result2.state).toBe("completed");
				expect(result2.output).toEqual(["first"]);
				return;
			}

			await handle.message("batch", "first");
			const result = await handle.result;
			expect(result.state).toBe("completed");
			expect(result.output).toEqual(["first"]);
		});

			it("should respect limits and FIFO ordering in queue.next", async () => {
			await driver.set(
				buildMessageKey("1"),
				serializeMessage(buildMessagePayload("fifo", "first", "1")),
			);
			await driver.set(
				buildMessageKey("2"),
				serializeMessage(buildMessagePayload("fifo", "second", "2")),
			);
			await driver.set(
				buildMessageKey("3"),
				serializeMessage(buildMessagePayload("fifo", "third", "3")),
			);

				const workflow = async (ctx: WorkflowContextInterface) => {
					const messages = await ctx.queue.next<string>("fifo", {
						names: ["fifo"],
						count: 2,
						timeout: 1000,
					});
					return messages.map((message) => message.body);
				};

			const result = await runWorkflow(
				"wf-1",
				workflow,
				undefined,
				driver,
				{
					mode,
				},
			).result;

			expect(result.state).toBe("completed");
			expect(result.output).toEqual(["first", "second"]);
		});

		it("should not see messages sent during execution until next run", async () => {
			let resolveGate: (() => void) | null = null;
			let resolveStarted: (() => void) | null = null;
			const gate = new Promise<void>((resolve) => {
				resolveGate = () => resolve();
			});
			const started = new Promise<void>((resolve) => {
				resolveStarted = () => resolve();
			});

			const workflow = async (ctx: WorkflowContextInterface) => {
				resolveStarted?.();
				await ctx.step("gate", async () => {
					await gate;
					return "ready";
				});

					const [message] = await ctx.queue.next<string>("wait", {
						names: ["mid"],
					});
					if (!message) {
						throw new Error("Expected message");
					}
					return message.body;
				};

			const handle = runWorkflow("wf-1", workflow, undefined, driver, {
				mode,
			});
			await started;
			await handle.message("mid", "value");
			if (resolveGate) {
				(resolveGate as () => void)();
			}

			if (mode === "yield") {
				const result1 = await handle.result;
				expect(result1.state).toBe("sleeping");

				const result2 = await runWorkflow(
					"wf-1",
					workflow,
					undefined,
					driver,
					{ mode },
				).result;
				expect(result2.state).toBe("completed");
				expect(result2.output).toBe("value");
				return;
			}

			const result = await handle.result;
			expect(result.state).toBe("completed");
			expect(result.output).toBe("value");
		});
	});
}
