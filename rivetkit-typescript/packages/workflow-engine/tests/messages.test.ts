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
				const message = await ctx.listen<string>(
					"wait-message",
					"my-message",
				);
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

		it("should listen for any message in a name set", async () => {
			const workflow = async (ctx: WorkflowContextInterface) => {
				const message = await ctx.listen<string>("wait-many", [
					"first",
					"second",
				]);
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
				const message = await ctx.listen<string>(
					"wait-message",
					"my-message",
				);
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

		it("listen should return a completable message handle", async () => {
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
				const message = await ctx.listen<string>("wait-message", "my-message");
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

		it("should collect multiple messages with listenN", async () => {
			await driver.set(
				buildMessageKey("1"),
				serializeMessage(buildMessagePayload("batch", "a", "1")),
			);
			await driver.set(
				buildMessageKey("2"),
				serializeMessage(buildMessagePayload("batch", "b", "2")),
			);

			const workflow = async (ctx: WorkflowContextInterface) => {
				return await ctx.listenN<string>("batch-wait", "batch", 2);
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

		it("should time out listenWithTimeout", async () => {
			const workflow = async (ctx: WorkflowContextInterface) => {
				return await ctx.listenWithTimeout<string>(
					"timeout",
					"missing",
					50,
				);
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

		it("should return a message before listenUntil deadline", async () => {
			const messageId = generateId();
			await driver.set(
				buildMessageKey(messageId),
				serializeMessage(
					buildMessagePayload("deadline", "data", messageId),
				),
			);

			const workflow = async (ctx: WorkflowContextInterface) => {
				return await ctx.listenUntil<string>(
					"deadline",
					"deadline",
					Date.now() + 1000,
				);
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

		it("should wait for listenNWithTimeout messages", async () => {
			const workflow = async (ctx: WorkflowContextInterface) => {
				return await ctx.listenNWithTimeout<string>(
					"batch",
					"batch",
					2,
					5000,
				);
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

		it("should respect limits and FIFO ordering in listenNUntil", async () => {
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
				return await ctx.listenNUntil<string>(
					"fifo",
					"fifo",
					2,
					Date.now() + 1000,
				);
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

				const message = await ctx.listen<string>("wait", "mid");
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
