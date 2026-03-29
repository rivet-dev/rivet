import type { ManagedProcess } from "@secure-exec/core";
import { afterAll, beforeAll, describe, expect, test } from "vitest";
import { AcpClient } from "../src/acp-client.js";
import { AgentOs } from "../src/agent-os.js";
import type { JsonRpcNotification } from "../src/protocol.js";
import type {
	AgentCapabilities,
	PermissionRequest,
	SessionConfigOption,
	SessionModeState,
} from "../src/session.js";
import { Session, type SessionInitData } from "../src/session.js";
import { createStdoutLineIterable } from "../src/stdout-lines.js";

/**
 * Build a mock ACP adapter script that uses the given prefix for session IDs.
 * Sends session/update notifications and a request/permission notification during prompt.
 */
function buildMockAdapter(prefix: string): string {
	return `
let buffer = '';
let sessionCounter = 0;
const PREFIX = ${JSON.stringify(prefix)};

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
            agentInfo: { name: 'subscription-agent', version: '1.0.0' },
            agentCapabilities: { permissions: true },
          };
          break;

        case 'session/new':
          sessionCounter++;
          result = { sessionId: PREFIX + '-' + sessionCounter };
          break;

        case 'session/prompt': {
          const sid = (msg.params && msg.params.sessionId) || 'unknown';
          process.stdout.write(JSON.stringify({
            jsonrpc: '2.0',
            method: 'session/update',
            params: { sessionId: sid, type: 'status', text: 'Working...' },
          }) + '\\n');
          process.stdout.write(JSON.stringify({
            jsonrpc: '2.0',
            method: 'session/update',
            params: { sessionId: sid, type: 'text', text: 'Hello from agent' },
          }) + '\\n');
          process.stdout.write(JSON.stringify({
            jsonrpc: '2.0',
            method: 'request/permission',
            params: { sessionId: sid, permissionId: 'perm-001', description: 'Run shell command' },
          }) + '\\n');
          result = { sessionId: sid, status: 'complete' };
          break;
        }

        case 'request/permission':
          result = { accepted: true };
          break;

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
}

let mockCounter = 0;

/**
 * Register a session in the AgentOs internal map using the mock adapter.
 * Returns the sessionId so we can subscribe via AgentOs methods.
 */
async function registerMockSession(
	vm: AgentOs,
	scriptPath: string,
): Promise<{ sessionId: string; proc: ManagedProcess }> {
	mockCounter++;
	const prefix = `sub-${mockCounter}`;
	await vm.writeFile(scriptPath, buildMockAdapter(prefix));
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

	return { sessionId, proc };
}

// Use a single VM for all tests to avoid process kill corrupting the VM.
// Sessions are NOT closed between tests (only via vm.dispose at the end).
describe("AgentOs event subscription", () => {
	let vm: AgentOs;

	beforeAll(async () => {
		vm = await AgentOs.create();
	});

	afterAll(async () => {
		await vm.dispose();
	});

	test("onSessionEvent: handler receives session/update events during prompt", async () => {
		const { sessionId } = await registerMockSession(
			vm,
			"/tmp/sub-1.mjs",
		);

		const received: JsonRpcNotification[] = [];
		vm.onSessionEvent(sessionId, (event) => {
			received.push(event);
		});

		await vm.prompt(sessionId, "trigger events");

		expect(received.length).toBeGreaterThanOrEqual(2);
		expect(received[0].method).toBe("session/update");

		const texts = received.map(
			(e) => (e.params as { text: string }).text,
		);
		expect(texts).toContain("Working...");
		expect(texts).toContain("Hello from agent");
	}, 30_000);

	test("onPermissionRequest: handler receives permission requests during prompt", async () => {
		const { sessionId } = await registerMockSession(
			vm,
			"/tmp/sub-2.mjs",
		);

		const permissions: PermissionRequest[] = [];
		vm.onPermissionRequest(sessionId, (req) => {
			permissions.push(req);
		});

		await vm.prompt(sessionId, "trigger permission");

		expect(permissions.length).toBeGreaterThanOrEqual(1);
		expect(permissions[0].permissionId).toBe("perm-001");
		expect(permissions[0].description).toBe("Run shell command");
	}, 30_000);

	test("onSessionEvent: unsubscribe stops handler from firing", async () => {
		const { sessionId } = await registerMockSession(
			vm,
			"/tmp/sub-3.mjs",
		);

		const received: JsonRpcNotification[] = [];
		const unsub = vm.onSessionEvent(sessionId, (event) => {
			received.push(event);
		});

		await vm.prompt(sessionId, "first prompt");
		const countAfterFirst = received.length;
		expect(countAfterFirst).toBeGreaterThanOrEqual(2);

		unsub();

		await vm.prompt(sessionId, "second prompt");
		expect(received.length).toBe(countAfterFirst);
	}, 30_000);

	test("onPermissionRequest: unsubscribe stops handler from firing", async () => {
		const { sessionId } = await registerMockSession(
			vm,
			"/tmp/sub-4.mjs",
		);

		const permissions: PermissionRequest[] = [];
		const unsub = vm.onPermissionRequest(sessionId, (req) => {
			permissions.push(req);
		});

		await vm.prompt(sessionId, "first prompt");
		const countAfterFirst = permissions.length;
		expect(countAfterFirst).toBeGreaterThanOrEqual(1);

		unsub();

		await vm.prompt(sessionId, "second prompt");
		expect(permissions.length).toBe(countAfterFirst);
	}, 30_000);

	test("multiple handlers can be registered per session", async () => {
		const { sessionId } = await registerMockSession(
			vm,
			"/tmp/sub-5.mjs",
		);

		const handler1Events: JsonRpcNotification[] = [];
		const handler2Events: JsonRpcNotification[] = [];
		const handler3Perms: PermissionRequest[] = [];
		const handler4Perms: PermissionRequest[] = [];

		vm.onSessionEvent(sessionId, (event) => {
			handler1Events.push(event);
		});
		vm.onSessionEvent(sessionId, (event) => {
			handler2Events.push(event);
		});
		vm.onPermissionRequest(sessionId, (req) => {
			handler3Perms.push(req);
		});
		vm.onPermissionRequest(sessionId, (req) => {
			handler4Perms.push(req);
		});

		await vm.prompt(sessionId, "multi handler test");

		expect(handler1Events.length).toBeGreaterThanOrEqual(2);
		expect(handler2Events.length).toBe(handler1Events.length);

		expect(handler3Perms.length).toBeGreaterThanOrEqual(1);
		expect(handler4Perms.length).toBe(handler3Perms.length);
	}, 30_000);

	test("handlers fire for specific session only", async () => {
		const { sessionId: sid1 } =
			await registerMockSession(vm, "/tmp/sub-6a.mjs");
		const { sessionId: sid2 } =
			await registerMockSession(vm, "/tmp/sub-6b.mjs");

		const events1: JsonRpcNotification[] = [];
		const events2: JsonRpcNotification[] = [];

		vm.onSessionEvent(sid1, (event) => {
			events1.push(event);
		});
		vm.onSessionEvent(sid2, (event) => {
			events2.push(event);
		});

		await vm.prompt(sid1, "session 1 only");

		expect(events1.length).toBeGreaterThanOrEqual(2);
		expect(events2.length).toBe(0);

		await vm.prompt(sid2, "session 2 only");

		const events1CountBefore = events1.length;
		expect(events2.length).toBeGreaterThanOrEqual(2);
		expect(events1.length).toBe(events1CountBefore);
	}, 30_000);
});
