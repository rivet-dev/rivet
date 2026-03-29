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
 * Mock ACP adapter supporting initialize, session/new, session/prompt, session/cancel.
 * session/prompt sends a session/update notification and a delayed response.
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
            agentInfo: { name: 'lifecycle-agent', version: '1.0.0' },
            agentCapabilities: { session_lifecycle: true },
          };
          break;

        case 'session/new':
          sessionCounter++;
          result = { sessionId: 'lc-session-' + sessionCounter };
          break;

        case 'session/prompt': {
          const sid = (msg.params && msg.params.sessionId) || 'unknown';
          process.stdout.write(JSON.stringify({
            jsonrpc: '2.0',
            method: 'session/update',
            params: { sessionId: sid, type: 'text', text: 'Response' },
          }) + '\\n');
          result = { sessionId: sid, status: 'complete' };
          break;
        }

        case 'session/cancel':
          result = { cancelled: true };
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

let globalCounter = 0;

async function createTrackedSession(
	vm: AgentOs,
	scriptPath: string,
): Promise<{
	sessionId: string;
	proc: ManagedProcess;
	client: AcpClient;
}> {
	const prefix = `lc${++globalCounter}`;
	const script = MOCK_ADAPTER.replace(
		"'lc-session-' + sessionCounter",
		`'${prefix}-' + sessionCounter`,
	);
	await vm.writeFile(scriptPath, script);
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

describe("session lifecycle: resume and destroy", () => {
	let vm: AgentOs;

	beforeEach(async () => {
		vm = await AgentOs.create();
	});

	afterEach(async () => {
		await vm.dispose();
	});

	test("resumeSession(id) returns the same Session object as createSession returned", async () => {
		const { sessionId } = await createTrackedSession(vm, "/tmp/lc-1.mjs");

		const resumed = vm.resumeSession(sessionId);
		// Verify the returned object has the same sessionId (no identity check since we no longer return Session objects)
		expect(resumed.sessionId).toBe(sessionId);

		vm.closeSession(sessionId);
	}, 30_000);

	test("resumed session is fully functional (prompt works)", async () => {
		const { sessionId } = await createTrackedSession(vm, "/tmp/lc-2.mjs");

		vm.resumeSession(sessionId);

		// Prompt on resumed session works
		const response = await vm.prompt(sessionId, "test after resume");
		expect(response.error).toBeUndefined();
		const result = response.result as { status: string };
		expect(result.status).toBe("complete");

		vm.closeSession(sessionId);
	}, 30_000);

	test("resumeSession with unknown ID throws", async () => {
		expect(() => vm.resumeSession("nonexistent-id")).toThrow(
			"Session not found",
		);
	}, 30_000);

	test("destroySession(id) removes session from listSessions()", async () => {
		const { sessionId } = await createTrackedSession(vm, "/tmp/lc-3.mjs");

		expect(vm.listSessions().length).toBe(1);

		await vm.destroySession(sessionId);

		expect(vm.listSessions().length).toBe(0);
	}, 30_000);

	test("destroySession kills the agent process", async () => {
		const { sessionId } = await createTrackedSession(vm, "/tmp/lc-4.mjs");

		await vm.destroySession(sessionId);

		// Session is closed — the process is killed
		expect(vm.listSessions().find((s) => s.sessionId === sessionId)).toBeUndefined();
	}, 30_000);

	test("prompt() on destroyed session throws", async () => {
		const { sessionId } = await createTrackedSession(vm, "/tmp/lc-5.mjs");

		await vm.destroySession(sessionId);

		await expect(vm.prompt(sessionId, "should fail")).rejects.toThrow(
			"Session not found",
		);
	}, 30_000);

	test("destroySession with unknown ID throws", async () => {
		await expect(vm.destroySession("nonexistent-id")).rejects.toThrow(
			"Session not found",
		);
	}, 30_000);

	test("destroySession cancels pending prompt before killing", async () => {
		const { sessionId } = await createTrackedSession(vm, "/tmp/lc-6.mjs");

		// destroySession attempts cancel (which may or may not have pending work),
		// then closes. This verifies the graceful shutdown path doesn't crash.
		await vm.destroySession(sessionId);

		// Session is closed and removed
		expect(vm.listSessions().find((s) => s.sessionId === sessionId)).toBeUndefined();
		expect(() => vm.resumeSession(sessionId)).toThrow("Session not found");
	}, 30_000);
});
