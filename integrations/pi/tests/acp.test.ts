import { randomUUID } from "node:crypto";
import { mkdtemp, rm } from "node:fs/promises";
import { type AddressInfo, createServer } from "node:net";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
	AgentSideConnection,
	type Client,
	ClientSideConnection,
	ndJsonStream,
	type SessionNotification,
	type SessionUpdate,
} from "@agentclientprotocol/sdk";
import { fauxAssistantMessage } from "@earendil-works/pi-ai";
import { setup } from "rivetkit";
import { createClient } from "rivetkit/client";
import { setupTest } from "rivetkit/test";
import { afterAll, beforeAll, describe, expect, type TestContext, test, vi } from "vitest";
import {
	PiAcpAgent,
	type PiAcpOptions,
	type PiAgentConnection,
	type PiAgentHandle,
} from "../src/acp/agent.js";
import { pi } from "../src/index.js";
import { localSandboxProvider } from "./helpers/local-sandbox.js";
import { createMockModel, type MockModel, slowly, toolCall } from "./helpers/mock-model.js";
import { startTcpProxy } from "./helpers/tcp-proxy.js";


let mockModel: MockModel;
let workdir: string;
let registry: ReturnType<typeof buildRegistry>;

function buildRegistry(mock: MockModel, root: string) {
	const models = {
		providers: { mock: mock.providerConfig },
		model: "mock/mock-model",
		scopedModels: ["mock/mock-model", "mock/mock-model-2"],
	};
	const agent = pi({
		...models,
		apiKeys: { mock: "key" },
		tools: ["write", "bash"],
		sandbox: localSandboxProvider(join(root, "sandboxes")),
		settings: { retry: { baseDelayMs: 10 } },
	});
	const loggedOut = pi({
		...models,
		credentials: () => ({
			list: async () => [],
			read: async () => undefined,
			refresh: async () => undefined,
		}),
	});
	return setup({ use: { agent, loggedOut } });
}

beforeAll(async () => {
	mockModel = createMockModel();
	workdir = await mkdtemp(join(tmpdir(), "rivet-pi-acp-test-"));
	registry = buildRegistry(mockModel, workdir);
	mockModel.reply(
		"create hello.txt",
		toolCall("write", { path: "hello.txt", content: "hi from pi\n" }),
		fauxAssistantMessage("written"),
	);
	mockModel.reply("answer slowly", slowly(fauxAssistantMessage("word ".repeat(400))));
	mockModel.reply("say hello", fauxAssistantMessage("Hi there!"));
	mockModel.reply(
		"answer despite overload",
		fauxAssistantMessage("", { stopReason: "error", errorMessage: "503 overloaded" }),
		fauxAssistantMessage("recovered"),
	);
	mockModel.reply(
		"fail at the model",
		fauxAssistantMessage("", { stopReason: "error", errorMessage: "400 credit balance is too low" }),
	);
});

afterAll(async () => {
	if (workdir) await rm(workdir, { recursive: true, force: true });
});

type Accessor = {
	getOrCreate(key: string[]): PiAgentHandle;
	get(key: string[]): PiAgentHandle;
};

/**
 * An ACP client connected to a `PiAcpAgent` over in-memory JSON-RPC streams,
 * the same protocol an editor speaks over stdio.
 */
function startEditor(
	actors: Accessor | PiAcpOptions["actor"],
	options: { login?: boolean; openTimeoutMs?: number; promptDelayMs?: number } = {},
) {
	const toAgent = new TransformStream<Uint8Array, Uint8Array>();
	const toClient = new TransformStream<Uint8Array, Uint8Array>();
	const updates: SessionNotification[] = [];
	const connections: PiAgentConnection[] = [];
	const recorded = (handle: PiAgentHandle): PiAgentHandle => ({
		getSession: () => handle.getSession(),
		connect: () => {
			const conn = handle.connect();
			connections.push(conn);
			const delayMs = options.promptDelayMs;
			if (!delayMs) return conn;
			return new Proxy(conn, {
				get: (target, name) =>
					name === "prompt"
						? async (...args: Parameters<PiAgentConnection["prompt"]>) => {
								await new Promise((resolve) => setTimeout(resolve, delayMs));
								return target.prompt(...args);
							}
						: Reflect.get(target, name),
			});
		},
	});
	const agentSide = new AgentSideConnection(
		(conn) =>
			new PiAcpAgent(conn, {
				actor: async (sessionId, create) =>
					recorded(
						typeof actors === "function"
							? await actors(sessionId, create)
							: create
								? actors.getOrCreate(["me", sessionId])
								: actors.get(["me", sessionId]),
					),
				login: options.login
					? { command: "node", commandArgs: ["rivet-pi.js", "acp"], loginArgs: ["--login"] }
					: undefined,
				openTimeoutMs: options.openTimeoutMs,
			}),
		ndJsonStream(toClient.writable, toAgent.readable),
	);
	const client: Client = {
		sessionUpdate: async (notification) => {
			updates.push(notification);
		},
		requestPermission: async () => ({ outcome: { outcome: "cancelled" } }),
	};
	const editor = new ClientSideConnection(() => client, ndJsonStream(toAgent.writable, toClient.readable));
	return { editor, agentSide, updates, connections };
}

function updatesOf(updates: SessionNotification[], sessionId: string): SessionUpdate[] {
	return updates.filter((n) => n.sessionId === sessionId).map((n) => n.update);
}

function text(updates: SessionUpdate[], kind: "agent_message_chunk" | "user_message_chunk"): string {
	return updates
		.filter((update) => update.sessionUpdate === kind)
		.map((update) => (update as { content: { text?: string } }).content.text ?? "")
		.join("");
}

describe("pi ACP bridge", () => {
	test("a prompt shows the tool call with its diff and ends with the turn", async (c) => {
		const { client } = await setupTest(c, registry);
		const { editor, updates } = startEditor(client.agent as unknown as Accessor);
		await editor.initialize({ protocolVersion: 1, clientCapabilities: {} });
		const { sessionId, configOptions } = await editor.newSession({ cwd: "/tmp", mcpServers: [] });
		expect(configOptions?.find((option) => option.id === "model")).toMatchObject({
			currentValue: "mock/mock-model",
		});

		const response = await editor.prompt({
			sessionId,
			prompt: [{ type: "text", text: "create hello.txt" }],
		});

		expect(response.stopReason).toBe("end_turn");
		const session = updatesOf(updates, sessionId);
		expect(session).toContainEqual(
			expect.objectContaining({
				sessionUpdate: "tool_call_update",
				status: "completed",
				content: [{ type: "diff", path: "hello.txt", oldText: null, newText: "hi from pi\n" }],
			}),
		);
		expect(text(session, "agent_message_chunk")).toContain("written");
	});

	test("cancel ends a turn as cancelled whether or not its run has started, and the session takes the next prompt", async (c) => {
		const { client } = await setupTest(c, registry);
		const { editor, updates } = startEditor(client.agent as unknown as Accessor, { promptDelayMs: 200 });
		await editor.initialize({ protocolVersion: 1, clientCapabilities: {} });
		const { sessionId } = await editor.newSession({ cwd: "/tmp", mcpServers: [] });

		const early = editor.prompt({ sessionId, prompt: [{ type: "text", text: "answer slowly" }] });
		await editor.cancel({ sessionId });
		expect((await early).stopReason).toBe("cancelled");
		const messages = await client.agent.get(["me", sessionId]).getMessages();
		expect(messages.at(-1)).toMatchObject({ role: "assistant", stopReason: "aborted" });

		const running = editor.prompt({ sessionId, prompt: [{ type: "text", text: "answer slowly" }] });
		// Cancel only after the first token, so the abort hits an in-flight response.
		await vi.waitFor(() => {
			expect(text(updatesOf(updates, sessionId), "agent_message_chunk")).not.toBe("");
		});
		await editor.cancel({ sessionId });
		expect((await running).stopReason).toBe("cancelled");

		const next = await editor.prompt({ sessionId, prompt: [{ type: "text", text: "say hello" }] });
		expect(next.stopReason).toBe("end_turn");
	});

	test("a final model failure fails the turn, and one that Pi retries successfully ends it normally", async (c) => {
		const { client } = await setupTest(c, registry);
		const { editor, updates } = startEditor(client.agent as unknown as Accessor);
		await editor.initialize({ protocolVersion: 1, clientCapabilities: {} });
		const { sessionId } = await editor.newSession({ cwd: "/tmp", mcpServers: [] });

		const response = await editor.prompt({ sessionId, prompt: [{ type: "text", text: "answer despite overload" }] });

		expect(response.stopReason).toBe("end_turn");
		expect(text(updatesOf(updates, sessionId), "agent_message_chunk")).toContain("recovered");

		await expect(
			editor.prompt({ sessionId, prompt: [{ type: "text", text: "fail at the model" }] }),
		).rejects.toMatchObject({ message: expect.stringContaining("credit balance is too low") });
	});

	test("loading a session in a new editor replays its history", async (c) => {
		const { client } = await setupTest(c, registry);
		const first = startEditor(client.agent as unknown as Accessor);
		await first.editor.initialize({ protocolVersion: 1, clientCapabilities: {} });
		const { sessionId } = await first.editor.newSession({ cwd: "/tmp", mcpServers: [] });
		await first.editor.prompt({ sessionId, prompt: [{ type: "text", text: "create hello.txt" }] });

		const second = startEditor(client.agent as unknown as Accessor);
		await second.editor.initialize({ protocolVersion: 1, clientCapabilities: {} });
		await second.editor.loadSession({ sessionId, cwd: "/tmp", mcpServers: [] });

		const replayed = updatesOf(second.updates, sessionId);
		expect(text(replayed, "user_message_chunk")).toBe("create hello.txt");
		expect(replayed).toContainEqual(
			expect.objectContaining({ sessionUpdate: "tool_call", title: "write hello.txt", kind: "edit" }),
		);
		expect(text(replayed, "agent_message_chunk")).toContain("written");

		await expect(
			second.editor.loadSession({ sessionId: randomUUID(), cwd: "/tmp", mcpServers: [] }),
		).rejects.toMatchObject({ code: -32002 });
	});

	test("the editor can switch to an allowed model but not to one outside the allowlist", async (c) => {
		const { client } = await setupTest(c, registry);
		const { editor } = startEditor(client.agent as unknown as Accessor);
		await editor.initialize({ protocolVersion: 1, clientCapabilities: {} });
		const { sessionId } = await editor.newSession({ cwd: "/tmp", mcpServers: [] });

		const switched = await editor.setSessionConfigOption({
			sessionId,
			configId: "model",
			value: "mock/mock-model-2",
		});
		expect(switched.configOptions.find((option) => option.id === "model")).toMatchObject({
			currentValue: "mock/mock-model-2",
		});
		await expect(
			editor.setSessionConfigOption({ sessionId, configId: "model", value: "mock/mock-model-3" }),
		).rejects.toThrow();
	});

	test("a new session asks the editor to log in when the user has no credentials", async (c) => {
		const { client } = await setupTest(c, registry);
		const { editor } = startEditor(client.loggedOut as unknown as Accessor, { login: true });
		const init = await editor.initialize({ protocolVersion: 1, clientCapabilities: {} });
		expect(init.authMethods).toEqual([
			expect.objectContaining({
				type: "terminal",
				args: ["--login"],
				_meta: { "terminal-auth": { command: "node", args: ["rivet-pi.js", "acp", "--login"], label: "Log in" } },
			}),
		]);

		await expect(editor.newSession({ cwd: "/tmp", mcpServers: [] })).rejects.toMatchObject({
			code: -32000,
			message: expect.stringContaining("Run `node rivet-pi.js acp --login` in a terminal."),
		});
	});

	test("a new session fails with an error instead of hanging when the engine is unreachable", async () => {
		const port = await closedPort();
		const client = createClient<typeof registry>({ endpoint: `http://127.0.0.1:${port}` });
		const { editor } = startEditor(client.agent as unknown as Accessor, { openTimeoutMs: 500 });
		await editor.initialize({ protocolVersion: 1, clientCapabilities: {} });

		await expect(editor.newSession({ cwd: "/tmp", mcpServers: [] })).rejects.toMatchObject({
			code: -32603,
			message: expect.stringContaining("Check that the Rivet engine at RIVET_ENDPOINT is running"),
		});
	});

	test("after the connection to the actor is lost between prompts, the next prompt still runs", async (c) => {
		const { client: direct } = await setupTest(c, registry);
		const { client, proxy } = await proxiedClient(c);
		const { editor, connections } = startEditor(client.agent as unknown as Accessor);
		await editor.initialize({ protocolVersion: 1, clientCapabilities: {} });
		const { sessionId } = await editor.newSession({ cwd: "/tmp", mcpServers: [] });

		const stopped = statusReached(connections[0]!, "idle");
		proxy.down();
		await stopped;
		proxy.up();

		const response = await editor.prompt({ sessionId, prompt: [{ type: "text", text: "say hello" }] });
		expect(response.stopReason).toBe("end_turn");
		expect(await direct.agent.get(["me", sessionId]).getLastAssistantText()).toBe("Hi there!");
	});

	test("a turn running when the connection is lost ends with an error instead of hanging", async (c) => {
		await setupTest(c, registry);
		const { client, proxy } = await proxiedClient(c);
		const { editor, updates } = startEditor(client.agent as unknown as Accessor);
		await editor.initialize({ protocolVersion: 1, clientCapabilities: {} });
		const { sessionId } = await editor.newSession({ cwd: "/tmp", mcpServers: [] });

		const running = editor.prompt({ sessionId, prompt: [{ type: "text", text: "answer slowly" }] });
		// Drop the connection only after the first token, so the run is in flight.
		await vi.waitFor(() => {
			expect(text(updatesOf(updates, sessionId), "agent_message_chunk")).not.toBe("");
		});
		proxy.down();

		await expect(running).rejects.toMatchObject({
			message: expect.stringContaining("Lost the connection to the Pi actor"),
		});
	});

	test("an editor that reaches each conversation with a scoped token keeps working after the token expires", async (c) => {
		const { client: server } = await setupTest(c, registry);
		const config = registry.parseConfig();
		const owned = new Set<string>();
		const issue = async (sessionId: string, create: boolean) => {
			if (create) owned.add(sessionId);
			if (!owned.has(sessionId)) throw new Error("You do not have access to this conversation.");
			const handle = server.agent.getOrCreate(["me", sessionId]);
			const { token } = await handle.issueToken({ subject: "me", expiresIn: 2 });
			return { actorId: await handle.resolve(), token };
		};
		const { editor, connections } = startEditor(async (sessionId, create) => {
			const { actorId } = await issue(sessionId, create);
			const client = createClient<typeof registry>({
				endpoint: config.endpoint!,
				namespace: config.namespace,
				poolName: config.envoy.poolName,
				getToken: async () => (await issue(sessionId, false)).token,
				disableMetadataLookup: true,
			});
			c.onTestFinished(() => client.dispose());
			return client.agent.getForId(actorId) as unknown as PiAgentHandle;
		});
		await editor.initialize({ protocolVersion: 1, clientCapabilities: {} });
		const { sessionId } = await editor.newSession({ cwd: "/tmp", mcpServers: [] });
		const first = await editor.prompt({ sessionId, prompt: [{ type: "text", text: "say hello" }] });
		expect(first.stopReason).toBe("end_turn");

		const conn = connections[0]!;
		await statusReached(conn, "connecting");
		await statusReached(conn, "connected");

		const second = await editor.prompt({ sessionId, prompt: [{ type: "text", text: "say hello" }] });
		expect(second.stopReason).toBe("end_turn");
		const messages = await server.agent.get(["me", sessionId]).getMessages();
		expect(messages.filter((message) => message.role === "user")).toHaveLength(2);

		await expect(
			editor.loadSession({ sessionId: randomUUID(), cwd: "/tmp", mcpServers: [] }),
		).rejects.toMatchObject({ message: expect.stringContaining("You do not have access to this conversation.") });
	});
});

/** A client that reaches the test engine through a proxy the test can take down. */
async function proxiedClient(c: TestContext) {
	const config = registry.parseConfig();
	const proxy = await startTcpProxy(config.endpoint!);
	c.onTestFinished(() => proxy.close());
	const client = createClient<typeof registry>({
		endpoint: proxy.url,
		namespace: config.namespace,
		poolName: config.envoy.poolName,
		token: config.token,
		disableMetadataLookup: true,
	});
	c.onTestFinished(() => client.dispose());
	return { client, proxy };
}

function statusReached(conn: PiAgentConnection, status: string): Promise<void> {
	return new Promise((resolve) => {
		conn.onStatusChange((current) => {
			if (current === status) resolve();
		});
	});
}

/** A local port with nothing listening on it. */
async function closedPort(): Promise<number> {
	const server = createServer();
	await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
	const { port } = server.address() as AddressInfo;
	await new Promise((resolve) => server.close(resolve));
	return port;
}
