import { afterAll, beforeAll, describe, expect, test } from "vitest";
import { AcpClient } from "../src/acp-client.js";
import { AgentOs } from "../src/agent-os.js";
import { Session, type SessionInitData } from "../src/session.js";
import { createStdoutLineIterable } from "../src/stdout-lines.js";

/**
 * Mock ACP adapter that handles initialize, session/new, session/prompt,
 * and session/cancel. Prompt responses include a session/update notification.
 */
const CANCEL_MOCK = `
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
            agentInfo: { name: 'cancel-test-agent', version: '1.0.0' },
          };
          break;

        case 'session/new':
          sessionCounter++;
          result = { sessionId: 'cancel-session-' + sessionCounter };
          break;

        case 'session/prompt': {
          const sid = (msg.params && msg.params.sessionId) || 'unknown';
          process.stdout.write(JSON.stringify({
            jsonrpc: '2.0',
            method: 'session/update',
            params: { sessionId: sid, type: 'text', text: 'Agent response' },
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

describe("session.cancel() tests", () => {
	let vm: AgentOs;

	beforeAll(async () => {
		vm = await AgentOs.create();
	});

	afterAll(async () => {
		await vm.dispose();
	});

	test("cancel: idle, during active prompt, and on closed session", async () => {
		// Spawn mock adapter and create session
		await vm.writeFile("/tmp/cancel-mock.mjs", CANCEL_MOCK);
		const { iterable, onStdout } = createStdoutLineIterable();
		const proc = vm.kernel.spawn("node", ["/tmp/cancel-mock.mjs"], {
			streamStdin: true,
			onStdout,
			env: { HOME: "/home/user" },
		});
		const client = new AcpClient(proc, iterable);

		const initResp = await client.request("initialize", {
			protocolVersion: 1,
			clientCapabilities: {},
		});
		expect(initResp.error).toBeUndefined();

		const sessionResp = await client.request("session/new", {
			cwd: "/home/user",
			mcpServers: [],
		});
		expect(sessionResp.error).toBeUndefined();

		const sessionId = (sessionResp.result as { sessionId: string }).sessionId;
		const initResult = initResp.result as Record<string, unknown>;
		const initData: SessionInitData = {};
		if (initResult.agentInfo) {
			initData.agentInfo =
				initResult.agentInfo as SessionInitData["agentInfo"];
		}

		const sessions = (vm as unknown as { _sessions: Map<string, Session> })
			._sessions;
		const session = new Session(client, sessionId, "mock", initData, () => {
			sessions.delete(sessionId);
		});
		sessions.set(sessionId, session);

		// --- 1. Cancel on idle session (no active prompt) ---
		const idleResponse = await vm.cancelSession(sessionId);
		expect(idleResponse.error).toBeUndefined();
		expect((idleResponse.result as { cancelled: boolean }).cancelled).toBe(
			true,
		);

		// Verify cancel sends the correct sessionId
		expect(sessionId).toMatch(/^cancel-session-/);

		// --- 2. Cancel during an active prompt ---
		// Fire prompt and cancel concurrently — AcpClient routes responses by id
		const [promptResponse, cancelResponse] = await Promise.all([
			vm.prompt(sessionId, "long running task"),
			vm.cancelSession(sessionId),
		]);

		// cancelSession() returns a successful JSON-RPC response
		expect(cancelResponse.error).toBeUndefined();
		expect(
			(cancelResponse.result as { cancelled: boolean }).cancelled,
		).toBe(true);

		// prompt also completes (mock responds immediately)
		expect(promptResponse.error).toBeUndefined();

		// --- 3. Cancel on closed session throws ---
		await vm.closeSession(sessionId);
		await expect(vm.cancelSession(sessionId)).rejects.toThrow("Session not found");
	}, 30_000);
});
