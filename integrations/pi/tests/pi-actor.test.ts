import { randomUUID } from "node:crypto";
import { mkdtemp, readFile, rm, stat } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fauxAssistantMessage } from "@earendil-works/pi-ai";
import type { AgentSessionEvent } from "@earendil-works/pi-coding-agent";
import { setup } from "rivetkit";
import { setupTest } from "rivetkit/test";
import { afterAll, beforeAll, describe, expect, test, vi } from "vitest";
import { pi } from "../src/index.js";
import { localSandboxProvider } from "./helpers/local-sandbox.js";
import { type MockModel, slowly, createMockModel, toolCall } from "./helpers/mock-model.js";

let mockModel: MockModel;
let workdir: string;
let registry: ReturnType<typeof buildRegistry>;

function buildRegistry(mock: MockModel, root: string) {
	const agent = pi({
		model: "mock/mock-model",
		providers: { mock: mock.providerConfig },
		apiKeys: { mock: "mock" },
		state: { sleeps: 0 },
		onSleep: (c) => {
			c.state.sleeps += 1;
		},
		actions: {
			nap: (c) => {
				c.sleep();
			},
			sleeps: (c) => c.state.sleeps,
		},
	});
	const sandboxed = pi({
		model: "mock/mock-model",
		providers: { mock: mock.providerConfig },
		apiKeys: { mock: "mock" },
		tools: ["write", "find", "bash"],
		sandbox: localSandboxProvider(join(root, "sandboxes")),
		state: { sleeps: 0 },
		onSleep: (c) => {
			c.state.sleeps += 1;
		},
		actions: {
			nap: (c) => {
				c.sleep();
			},
			sleeps: (c) => c.state.sleeps,
			destroySelf: (c) => {
				c.destroy();
			},
		},
	});
	const timed = pi({
		model: "mock/mock-model",
		providers: { mock: mock.providerConfig },
		apiKeys: { mock: "mock" },
		options: { actionTimeout: 1_000 },
	});
	return setup({ use: { agent, sandboxed, timed } });
}

beforeAll(async () => {
	mockModel = createMockModel();
	workdir = await mkdtemp(join(tmpdir(), "rivet-pi-test-"));
	registry = buildRegistry(mockModel, workdir);

	mockModel.reply("say hello", fauxAssistantMessage("Hi there!"));
	mockModel.reply("answer slowly", slowly(fauxAssistantMessage("word ".repeat(400))));
	mockModel.reply(
		"create hello.txt",
		toolCall("write", { path: "hello.txt", content: "hi from pi\n" }),
		fauxAssistantMessage("written"),
	);
	mockModel.reply("find ts files", toolCall("find", { pattern: "*.ts" }), fauxAssistantMessage("listed"));
	mockModel.reply("run a slow command", toolCall("bash", { command: "sleep 30" }), fauxAssistantMessage("ran"));
	mockModel.reply(
		"run with a timeout",
		toolCall("bash", { command: "sleep 5", timeout: 1 }),
		fauxAssistantMessage("ran"),
	);
	mockModel.reply(
		"print the host key",
		toolCall("bash", { command: 'echo "key=[$PI_TEST_HOST_KEY]"' }),
		fauxAssistantMessage("ran"),
	);
});

afterAll(async () => {
	if (workdir) await rm(workdir, { recursive: true, force: true });
});

function collectEvents(conn: {
	on: (name: "event", callback: (event: AgentSessionEvent) => void) => unknown;
}) {
	const events: AgentSessionEvent[] = [];
	conn.on("event", (event) => {
		events.push(event);
	});
	return events;
}

describe("pi actor", () => {
	test("the same session continues after the actor sleeps", async (c) => {
		const { client } = await setupTest(c, registry);
		const handle = client.agent.getOrCreate(["sleeps", randomUUID()]);
		await handle.prompt("say hello");
		const before = await handle.getSession();
		const messagesBefore = await handle.getMessages();

		await handle.nap();
		// Sleep finishes after the `nap` action returns, so poll the persisted
		// counter until the sleep hook has run.
		await vi.waitFor(async () => {
			expect(await handle.sleeps()).toBeGreaterThanOrEqual(1);
		});

		const after = await handle.getSession();
		expect(after.sessionId).toBe(before.sessionId);
		expect(after.cwd).toBe(before.cwd);
		expect(await handle.getMessages()).toHaveLength(messagesBefore.length);

		await handle.prompt("say hello");
		expect((await handle.getMessages()).length).toBeGreaterThan(
			messagesBefore.length,
		);
	});

	test("abort stops a streaming run and records it as aborted", async (c) => {
		const { client } = await setupTest(c, registry);
		const conn = client.agent.getOrCreate(["aborts", randomUUID()]).connect();
		const events = collectEvents(conn);
		await conn.getSession();

		const running = conn.prompt("answer slowly");
		// Wait for the first token so the abort hits an in-flight response.
		await vi.waitFor(() => {
			expect(events.some((event) => event.type === "message_update")).toBe(true);
		});
		await conn.abort();
		await expect(running).resolves.toBeUndefined();

		const messages = await conn.getMessages();
		const assistant = [...messages]
			.reverse()
			.find((message) => message.role === "assistant");
		expect(assistant).toMatchObject({ stopReason: "aborted" });
		expect((await conn.getSession()).isStreaming).toBe(false);
		await conn.dispose();
	});

	test("a prompt that outlives the action timeout rejects and its run stops", async (c) => {
		const { client } = await setupTest(c, registry);
		const handle = client.timed.getOrCreate(["times-out", randomUUID()]);

		await expect(handle.prompt("answer slowly")).rejects.toMatchObject({ code: "action_timed_out" });

		await handle.waitForIdle();
		const messages = await handle.getMessages();
		expect(messages.at(-1)).toMatchObject({ role: "assistant", stopReason: "aborted" });
	});

	test("a second prompt during a run is rejected and does not start a run", async (c) => {
		const { client } = await setupTest(c, registry);
		const conn = client.agent.getOrCreate(["overlaps", randomUUID()]).connect();
		const events = collectEvents(conn);
		await conn.getSession();

		const running = conn.prompt("answer slowly");
		// The second prompt must arrive while the first run is streaming.
		await vi.waitFor(() => {
			expect(events.some((event) => event.type === "agent_start")).toBe(true);
		});
		await expect(conn.prompt("say hello")).rejects.toThrow();
		await conn.abort();
		await running;

		expect(events.filter((event) => event.type === "agent_start")).toHaveLength(1);
		await conn.dispose();
	});

	test("built-in tools run inside the sandbox", async (c) => {
		const { client } = await setupTest(c, registry);
		const conn = client.sandboxed.getOrCreate(["sandboxed", randomUUID()]).connect();
		const events = collectEvents(conn);
		const { cwd } = await conn.getSession();

		await conn.prompt("create hello.txt");
		expect(await readFile(join(cwd, "hello.txt"), "utf8")).toBe("hi from pi\n");

		await conn.executeBash("mkdir -p src && touch src/nested.ts");
		await conn.prompt("find ts files");
		expect(JSON.stringify(toolResult(events, "find"))).toContain("src/nested.ts");
		await conn.dispose();
	});

	test("abort stops a running sandbox command without waiting for it", async (c) => {
		const { client } = await setupTest(c, registry);
		const conn = client.sandboxed.getOrCreate(["kills", randomUUID()]).connect();
		const events = collectEvents(conn);
		await conn.getSession();

		const running = conn.prompt("run a slow command");
		// Wait until the tool has started so the abort hits a running command.
		await vi.waitFor(() => {
			expect(
				events.some(
					(event) => event.type === "tool_execution_start" && event.toolName === "bash",
				),
			).toBe(true);
		});
		const started = Date.now();
		await conn.abort();
		await running;

		expect(Date.now() - started).toBeLessThan(5_000);
		expect(JSON.stringify(toolResult(events, "bash"))).toContain("Command aborted");
		await conn.dispose();
	});

	test("a bash timeout is measured in seconds", async (c) => {
		const { client } = await setupTest(c, registry);
		const conn = client.sandboxed.getOrCreate(["timeouts", randomUUID()]).connect();
		const events = collectEvents(conn);
		await conn.getSession();

		await conn.prompt("run with a timeout");

		expect(JSON.stringify(toolResult(events, "bash"))).toContain(
			"Command timed out after 1 seconds",
		);
		await conn.dispose();
	});

	test("a sandbox command cannot read the actor host's environment", async (c) => {
		process.env.PI_TEST_HOST_KEY = "host-key";
		c.onTestFinished(() => {
			delete process.env.PI_TEST_HOST_KEY;
		});
		const { client } = await setupTest(c, registry);
		const conn = client.sandboxed.getOrCreate(["host-env", randomUUID()]).connect();
		const events = collectEvents(conn);
		await conn.getSession();

		await conn.prompt("print the host key");

		const output = JSON.stringify(toolResult(events, "bash"));
		expect(output).toContain("key=[]");
		expect(output).not.toContain("host-key");
		await conn.dispose();
	});

	test("the sandbox outlives sleep, is replaced when deleted, and is destroyed with the actor", async (c) => {
		const { client } = await setupTest(c, registry);
		const handle = client.sandboxed.getOrCreate(["lifecycle", randomUUID()]);
		await handle.executeBash("echo kept > kept.txt");
		const first = await handle.getSession();

		await handle.nap();
		// Sleep finishes after the `nap` action returns, so poll the persisted
		// counter until the sleep hook has run.
		await vi.waitFor(async () => {
			expect(await handle.sleeps()).toBe(1);
		});
		expect((await handle.executeBash("cat kept.txt")).output.trim()).toBe("kept");
		expect((await handle.getSession()).cwd).toBe(first.cwd);

		await handle.nap();
		// Same reason as above: wait for the second sleep hook.
		await vi.waitFor(async () => {
			expect(await handle.sleeps()).toBe(2);
		});
		await rm(first.cwd, { recursive: true, force: true });
		const replaced = await handle.getSession();
		expect(replaced.sessionId).toBe(first.sessionId);
		expect(replaced.cwd).not.toBe(first.cwd);
		expect((await handle.executeBash("test -e kept.txt")).exitCode).toBe(1);

		await handle.destroySelf();
		// Destruction runs after the action returns; poll the sandbox directory.
		await vi.waitFor(async () => {
			expect(await exists(replaced.cwd)).toBe(false);
		});
	});
});

function toolResult(events: AgentSessionEvent[], toolName: string) {
	return events.find(
		(event) => event.type === "tool_execution_end" && event.toolName === toolName,
	);
}

function exists(path: string): Promise<boolean> {
	return stat(path).then(
		() => true,
		() => false,
	);
}
