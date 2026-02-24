import { beforeEach, describe, expect, it } from "vitest";
import {
	generateId,
	InMemoryDriver,
	runWorkflow,
	type Message,
	type WorkflowContextInterface,
	type WorkflowMessageDriver,
} from "../src/testing.js";

async function queueMessage(
	driver: InMemoryDriver,
	name: string,
	data: unknown,
	id = generateId(),
): Promise<void> {
	await driver.messageDriver.addMessage({
		id,
		name,
		data,
		sentAt: Date.now(),
	});
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
			await queueMessage(driver, "my-message", "hello");

			const workflow = async (ctx: WorkflowContextInterface) => {
				const [message] = await ctx.queue.next<string>("wait-message", {
					names: ["my-message"],
				});
				if (!message) {
					throw new Error("Expected message");
				}
				return message.body;
			};

			const result = await runWorkflow("wf-1", workflow, undefined, driver, {
				mode,
			}).result;

			expect(result.state).toBe("completed");
			expect(result.output).toBe("hello");
		});

		it("queue.next should return completable messages", async () => {
			const completions: Array<{ id: string; response?: unknown }> = [];
			const pending: Message[] = [
				{
					id: "msg-1",
					name: "my-message",
					data: "hello",
					sentAt: Date.now(),
				},
			];

			const messageDriver: WorkflowMessageDriver = {
				async addMessage(message) {
					pending.push(message);
				},
				async receiveMessages(opts) {
					const nameSet =
						opts.names && opts.names.length > 0
							? new Set(opts.names)
							: undefined;
					const selected: Array<{ message: Message; index: number }> = [];
					for (let i = 0; i < pending.length && selected.length < opts.count; i++) {
						const message = pending[i];
						if (nameSet && !nameSet.has(message.name)) {
							continue;
						}
						selected.push({ message, index: i });
					}
					if (!opts.completable) {
						for (let i = selected.length - 1; i >= 0; i--) {
							pending.splice(selected[i].index, 1);
						}
						return selected.map((entry) => entry.message);
					}
					return selected.map((entry) => {
						const { message } = entry;
						return {
							...message,
							complete: async (response?: unknown) => {
								completions.push({ id: message.id, response });
								const index = pending.findIndex((m) => m.id === message.id);
								if (index !== -1) {
									pending.splice(index, 1);
								}
							},
						};
					});
				},
				async completeMessage(messageId, response) {
					completions.push({ id: messageId, response });
					const index = pending.findIndex((message) => message.id === messageId);
					if (index !== -1) {
						pending.splice(index, 1);
					}
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

			const result = await runWorkflow("wf-1", workflow, undefined, driver, {
				mode,
			}).result;

			expect(result.state).toBe("completed");
			expect(result.output).toBe("hello");
			expect(completions).toEqual([{ id: "msg-1", response: { ok: true } }]);
		});

		it("replay should not block the next completable queue.next", async () => {
			if (mode !== "yield") {
				return;
			}

			await queueMessage(driver, "my-message", "one", "msg-1");
			await queueMessage(driver, "my-message", "two", "msg-2");

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

			await queueMessage(driver, "my-message", "one", "msg-1");
			await queueMessage(driver, "my-message", "two", "msg-2");

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

			const secondRunHandle = runWorkflow("wf-1", workflow, undefined, driver, {
				mode,
			});
			await expect(secondRunHandle.result).rejects.toThrow(
				"Previous completable queue message is not completed.",
			);

			const queued = await driver.messageDriver.receiveMessages({
				names: ["my-message"],
				count: 10,
				completable: true,
			});
			expect(queued.map((message) => String(message.id)).sort()).toEqual([
				"msg-1",
				"msg-2",
			]);
		});

		it("should collect multiple messages with queue.next count", async () => {
			await queueMessage(driver, "batch", "a", "1");
			await queueMessage(driver, "batch", "b", "2");

			const workflow = async (ctx: WorkflowContextInterface) => {
				const messages = await ctx.queue.next<string>("batch-wait", {
					names: ["batch"],
					count: 2,
				});
				return messages.map((message) => message.body);
			};

			const result = await runWorkflow("wf-1", workflow, undefined, driver, {
				mode,
			}).result;

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

			const result = await runWorkflow("wf-1", workflow, undefined, driver, {
				mode,
			}).result;
			expect(result.state).toBe("completed");
			expect(result.output).toBeNull();
		});

		it("should return a message before queue.next timeout", async () => {
			await queueMessage(driver, "deadline", "data");

			const workflow = async (ctx: WorkflowContextInterface) => {
				const messages = await ctx.queue.next<string>("deadline", {
					names: ["deadline"],
					timeout: 1000,
				});
				return messages[0]?.body ?? null;
			};

			const result = await runWorkflow("wf-1", workflow, undefined, driver, {
				mode,
			}).result;

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
			await queueMessage(driver, "fifo", "first", "1");
			await queueMessage(driver, "fifo", "second", "2");
			await queueMessage(driver, "fifo", "third", "3");

			const workflow = async (ctx: WorkflowContextInterface) => {
				const messages = await ctx.queue.next<string>("fifo", {
					names: ["fifo"],
					count: 2,
					timeout: 1000,
				});
				return messages.map((message) => message.body);
			};

			const result = await runWorkflow("wf-1", workflow, undefined, driver, {
				mode,
			}).result;

			expect(result.state).toBe("completed");
			expect(result.output).toEqual(["first", "second"]);
		});

		it("should consume messages sent during execution", async () => {
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

			const result = await handle.result;
			expect(result.state).toBe("completed");
			expect(result.output).toBe("value");
		});
	});
}
