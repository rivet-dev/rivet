import type { ManagedProcess } from "@secure-exec/core";
import { afterEach, beforeEach, describe, expect, test } from "vitest";
import { AcpClient } from "../src/acp-client.js";
import { AgentOs } from "../src/agent-os.js";
import type {
	AgentCapabilities,
	SessionConfigOption,
	SessionModeState,
} from "../src/session.js";
import { Session, type SessionInitData } from "../src/session.js";
import { createStdoutLineIterable } from "../src/stdout-lines.js";

/**
 * Mock ACP adapter with rich capabilities in the initialize response.
 * Supports custom/echo to test rawSend, and returns -32601 for unknown methods.
 */
const MOCK_ADAPTER = `
let buffer = '';
let sessionCounter = 0;

process.stdin.resume();
process.stdin.on('data', (chunk) => {
  const str = chunk instanceof Uint8Array ? new TextDecoder().decode(chunk) : String(chunk);
  buffer += str;

  while (true) {
    const idx = buffer.indexOf('\\n');
    if (idx === -1) break;
    const line = buffer.substring(0, idx);
    buffer = buffer.substring(idx + 1);
    if (!line.trim()) continue;

    try {
      const msg = JSON.parse(line);
      if (msg.id === undefined) continue;

      let result;
      switch (msg.method) {
        case 'initialize':
          result = {
            protocolVersion: 1,
            agentInfo: { name: 'test-agent', version: '2.0.0' },
            agentCapabilities: {
              permissions: true,
              plan_mode: true,
              questions: false,
              tool_calls: true,
              text_messages: true,
              images: false,
              file_attachments: false,
              session_lifecycle: true,
              error_events: true,
              reasoning: true,
              status: true,
              streaming_deltas: false,
              mcp_tools: true,
            },
          };
          break;

        case 'session/new':
          sessionCounter++;
          result = { sessionId: 'cap-session-' + sessionCounter };
          break;

        case 'session/prompt':
          result = { sessionId: msg.params.sessionId, status: 'complete' };
          break;

        case 'custom/echo':
          result = { echo: msg.params };
          break;

        default:
          process.stdout.write(JSON.stringify({
            jsonrpc: '2.0', id: msg.id,
            error: { code: -32601, message: 'Method not found: ' + msg.method },
          }) + '\\n');
          continue;
      }

      process.stdout.write(JSON.stringify({
        jsonrpc: '2.0', id: msg.id, result,
      }) + '\\n');
    } catch (e) {}
  }
});
`;

async function createTrackedSession(
	vm: AgentOs,
	scriptPath: string,
): Promise<{
	sessionId: string;
	proc: ManagedProcess;
	client: AcpClient;
}> {
	await vm.writeFile(scriptPath, MOCK_ADAPTER);
	const { iterable, onStdout } = createStdoutLineIterable();
	const proc = vm.kernel.spawn("node", [scriptPath], {
		streamStdin: true,
		onStdout,
		env: { HOME: "/home/user" },
	});
	const client = new AcpClient(proc, iterable);

	const initResp = await client.request("initialize", {
		protocolVersion: 1,
		clientCapabilities: {},
	});
	if (initResp.error) {
		client.close();
		throw new Error(`initialize failed: ${initResp.error.message}`);
	}

	const sessionResp = await client.request("session/new", {
		cwd: "/home/user",
		mcpServers: [],
	});
	if (sessionResp.error) {
		client.close();
		throw new Error(`session/new failed: ${sessionResp.error.message}`);
	}

	const initResult = initResp.result as Record<string, unknown>;
	const sessionId = (sessionResp.result as { sessionId: string }).sessionId;

	const initData: SessionInitData = {};
	if (initResult.agentCapabilities) {
		initData.capabilities =
			initResult.agentCapabilities as AgentCapabilities;
	}
	if (initResult.agentInfo) {
		initData.agentInfo =
			initResult.agentInfo as SessionInitData["agentInfo"];
	}
	if (initResult.modes) {
		initData.modes = initResult.modes as SessionModeState;
	}
	if (initResult.configOptions) {
		initData.configOptions =
			initResult.configOptions as SessionConfigOption[];
	}

	const sessions = (vm as unknown as { _sessions: Map<string, Session> })
		._sessions;
	const session = new Session(client, sessionId, "mock", initData, () => {
		sessions.delete(sessionId);
	});
	sessions.set(sessionId, session);

	return { sessionId, proc, client };
}

describe("session capabilities and rawSend", () => {
	let vm: AgentOs;

	beforeEach(async () => {
		vm = await AgentOs.create();
	});

	afterEach(async () => {
		await vm.dispose();
	});

	test("session.capabilities returns object from initialize response", async () => {
		const { sessionId } = await createTrackedSession(vm, "/tmp/caps-1.mjs");

		const caps = vm.getSessionCapabilities(sessionId) as AgentCapabilities;
		expect(caps).toBeDefined();
		expect(typeof caps).toBe("object");
		// Verify the full object matches what the mock adapter sends
		expect(caps.permissions).toBe(true);
		expect(caps.plan_mode).toBe(true);
		expect(caps.tool_calls).toBe(true);
		expect(caps.text_messages).toBe(true);
		expect(caps.session_lifecycle).toBe(true);
		expect(caps.error_events).toBe(true);
		expect(caps.reasoning).toBe(true);
		expect(caps.status).toBe(true);
		expect(caps.mcp_tools).toBe(true);

		vm.closeSession(sessionId);
	}, 30_000);

	test("session.agentInfo has name and version from initialize", async () => {
		const { sessionId } = await createTrackedSession(vm, "/tmp/caps-2.mjs");

		const info = vm.getSessionAgentInfo(sessionId);
		expect(info).toBeDefined();
		expect(info?.name).toBe("test-agent");
		expect(info?.version).toBe("2.0.0");

		vm.closeSession(sessionId);
	}, 30_000);

	test("capabilities boolean flags are accessible (permissions, plan_mode, etc.)", async () => {
		const { sessionId } = await createTrackedSession(vm, "/tmp/caps-3.mjs");

		const caps = vm.getSessionCapabilities(sessionId) as AgentCapabilities;
		// True flags
		expect(caps.permissions).toBe(true);
		expect(caps.plan_mode).toBe(true);
		// False flags
		expect(caps.questions).toBe(false);
		expect(caps.images).toBe(false);
		expect(caps.file_attachments).toBe(false);
		expect(caps.streaming_deltas).toBe(false);

		vm.closeSession(sessionId);
	}, 30_000);

	test("rawSend sends arbitrary method and returns response", async () => {
		const { sessionId } = await createTrackedSession(vm, "/tmp/raw-1.mjs");

		const resp = await vm.rawSessionSend(sessionId, "custom/echo", { foo: "bar" });
		expect(resp.error).toBeUndefined();
		const result = resp.result as { echo: Record<string, unknown> };
		expect(result.echo.foo).toBe("bar");

		vm.closeSession(sessionId);
	}, 30_000);

	test("rawSend auto-injects sessionId into params", async () => {
		const { sessionId } = await createTrackedSession(vm, "/tmp/raw-2.mjs");

		// Send without sessionId — rawSend should inject it
		const resp = await vm.rawSessionSend(sessionId, "custom/echo", { data: "test" });
		expect(resp.error).toBeUndefined();
		const result = resp.result as { echo: Record<string, unknown> };
		// The echo response includes the params the adapter received
		expect(result.echo.sessionId).toBe(sessionId);
		expect(result.echo.data).toBe("test");

		vm.closeSession(sessionId);
	}, 30_000);

	test("rawSend with unknown method returns JSON-RPC error (not crash)", async () => {
		const { sessionId } = await createTrackedSession(vm, "/tmp/raw-3.mjs");

		const resp = await vm.rawSessionSend(sessionId, "nonexistent/method");
		expect(resp.error).toBeDefined();
		expect(resp.error?.code).toBe(-32601);
		expect(resp.error?.message).toContain("Method not found");

		vm.closeSession(sessionId);
	}, 30_000);

	test("rawSend on closed session throws", async () => {
		const { sessionId } = await createTrackedSession(vm, "/tmp/raw-4.mjs");

		vm.closeSession(sessionId);

		await expect(vm.rawSessionSend(sessionId, "custom/echo")).rejects.toThrow(
			"Session not found",
		);
	}, 30_000);
});
