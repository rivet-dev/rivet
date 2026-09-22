import { randomUUID } from "node:crypto";
import { mkdtemp, readFile, rm, stat } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import type { AgentSessionEvent } from "@earendil-works/pi-coding-agent";
import { setup } from "rivetkit";
import { setupTest } from "rivetkit/test";
import { afterAll, beforeAll, describe, expect, test, vi } from "vitest";
import { pi } from "../src/index.js";
import { localSandboxProvider } from "./helpers/local-sandbox.js";
import { type MockModel, startMockModel } from "./helpers/mock-model.js";

let mockModel: MockModel;
let workdir: string;
let registry: ReturnType<typeof buildRegistry>;

function buildRegistry(mock: MockModel, root: string) {
	const agent = pi({
		model: mock.model,
		modelRuntime: mock.modelRuntime,
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
		model: mock.model,
		modelRuntime: mock.modelRuntime,
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
	return setup({ use: { agent, sandboxed } });
}

beforeAll(async () => {
	mockModel = await startMockModel();
	workdir = await mkdtemp(join(tmpdir(), "rivet-pi-test-"));
	registry = buildRegistry(mockModel, workdir);

	mockModel.mock.onMessage("say hello", { content: "Hi there!" });
	mockModel.mock.onMessage(
		"answer slowly",
		{ content: "word ".repeat(400) },
		{ streamingProfile: { ttft: 20, tps: 25 } },
	);
	// Tool-result fixtures go first: the user message is still in the history
	// of the follow-up request, so they must win over the user-message fixtures.
	mockModel.mock.prependFixture({
		match: { hasToolResult: true, toolName: "write" },
		response: { content: "written" },
	});
	mockModel.mock.onMessage("create hello.txt", {
		toolCalls: [
			{ name: "write", arguments: { path: "hello.txt", content: "hi from pi\n" } },
		],
	});
	mockModel.mock.prependFixture({
		match: { hasToolResult: true, toolName: "find" },
		response: { content: "listed" },
	});
	mockModel.mock.prependFixture({
		match: { hasToolResult: true, toolName: "bash" },
		response: { content: "ran" },
	});
	mockModel.mock.onMessage("find ts files", {
		toolCalls: [{ name: "find", arguments: { pattern: "*.ts" } }],
	});
	mockModel.mock.onMessage("run a slow command", {
		toolCalls: [{ name: "bash", arguments: { command: "sleep 30" } }],
	});
	mockModel.mock.onMessage("run with a timeout", {
		toolCalls: [{ name: "bash", arguments: { command: "sleep 5", timeout: 1 } }],
	});
});

afterAll(async () => {
	await mockModel?.dispose();
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
		await handle.waitForIdle();
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
		await handle.waitForIdle();
		expect((await handle.getMessages()).length).toBeGreaterThan(
			messagesBefore.length,
		);
	});

	test("abort stops a streaming run and records it as aborted", async (c) => {
		const { client } = await setupTest(c, registry);
		const conn = client.agent.getOrCreate(["aborts", randomUUID()]).connect();
		const events = collectEvents(conn);
		await conn.getSession();

		await conn.prompt("answer slowly");
		// The run streams in the actor after `prompt` returns; wait for the
		// first token before aborting so the abort hits an in-flight response.
		await vi.waitFor(() => {
			expect(events.some((event) => event.type === "message_update")).toBe(true);
		});
		await conn.abort();
		await conn.waitForIdle();

		const messages = await conn.getMessages();
		const assistant = [...messages]
			.reverse()
			.find((message) => message.role === "assistant");
		expect(assistant).toMatchObject({ stopReason: "aborted" });
		expect((await conn.getSession()).isStreaming).toBe(false);
		await conn.dispose();
	});

	test("a second prompt during a run is rejected and does not start a run", async (c) => {
		const { client } = await setupTest(c, registry);
		const conn = client.agent.getOrCreate(["overlaps", randomUUID()]).connect();
		const events = collectEvents(conn);
		await conn.getSession();

		await conn.prompt("answer slowly");
		await expect(conn.prompt("say hello")).rejects.toThrow();
		await conn.abort();
		await conn.waitForIdle();

		expect(events.filter((event) => event.type === "agent_start")).toHaveLength(1);
		await conn.dispose();
	});

	test("built-in tools run inside the sandbox", async (c) => {
		const { client } = await setupTest(c, registry);
		const conn = client.sandboxed.getOrCreate(["sandboxed", randomUUID()]).connect();
		const events = collectEvents(conn);
		const { cwd } = await conn.getSession();

		await conn.prompt("create hello.txt");
		await conn.waitForIdle();
		expect(await readFile(join(cwd, "hello.txt"), "utf8")).toBe("hi from pi\n");

		await conn.executeBash("mkdir -p src && touch src/nested.ts");
		await conn.prompt("find ts files");
		await conn.waitForIdle();
		expect(JSON.stringify(toolResult(events, "find"))).toContain("src/nested.ts");
		await conn.dispose();
	});

	test("abort stops a running sandbox command without waiting for it", async (c) => {
		const { client } = await setupTest(c, registry);
		const conn = client.sandboxed.getOrCreate(["kills", randomUUID()]).connect();
		const events = collectEvents(conn);
		await conn.getSession();

		await conn.prompt("run a slow command");
		// The tool runs in the actor after `prompt` returns; wait until it has
		// started so the abort hits a running command.
		await vi.waitFor(() => {
			expect(
				events.some(
					(event) => event.type === "tool_execution_start" && event.toolName === "bash",
				),
			).toBe(true);
		});
		const started = Date.now();
		await conn.abort();
		await conn.waitForIdle();

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
		await conn.waitForIdle();

		expect(JSON.stringify(toolResult(events, "bash"))).toContain(
			"Command timed out after 1 seconds",
		);
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
