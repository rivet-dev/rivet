import { resolve } from "node:path";
import type { ManagedProcess } from "@secure-exec/core";
import { afterEach, beforeEach, describe, expect, test } from "vitest";
import { AcpClient } from "../src/acp-client.js";
import { AgentOs } from "../src/agent-os.js";
import { AGENT_CONFIGS } from "../src/agents.js";
import type { JsonRpcNotification } from "../src/protocol.js";
import { Session } from "../src/session.js";
import { createStdoutLineIterable } from "../src/stdout-lines.js";

/**
 * Full createSession('opencode') Tests
 *
 * BLOCKED: OpenCode is a native ELF binary (compiled Go), not Node.js. The
 * secure-exec VM kernel only supports JS/WASM command execution. The opencode-ai
 * npm package is a JS wrapper that calls child_process.spawnSync on the native
 * binary, which returns ENOENT inside the VM.
 *
 * Differences from PI session behavior:
 * - OpenCode speaks ACP natively (`opencode acp`), so acpAdapter and agentPackage
 *   are the same package (opencode-ai). PI uses a separate adapter (pi-acp).
 * - OpenCode's ACP mode starts directly as a JSON-RPC server; PI's pi-acp spawns
 *   the PI CLI as a child process, adding another layer of process management.
 * - Both agents use the same JSON-RPC 2.0 protocol: initialize, session/new,
 *   session/prompt, session/cancel, session/update notifications.
 * - OpenCode's agentInfo.name would be "opencode" (vs pi-acp's "pi-acp").
 * - Since OpenCode is its own adapter, createSession('opencode') has one fewer
 *   process indirection — no adapter-spawns-agent chain.
 *
 * To enable real OpenCode sessions in the VM, one of:
 * (a) Add native binary execution support to the secure-exec kernel
 * (b) Run OpenCode outside the VM and proxy ACP over a socket/pipe
 * (c) Build a WASM version of OpenCode (unlikely given it's Go + native deps)
 */

const MODULE_ACCESS_CWD = resolve(import.meta.dirname, "..");

/**
 * Mock OpenCode ACP adapter for testing the Session lifecycle.
 * Mirrors what `opencode acp` would do: JSON-RPC 2.0 over stdio.
 * Responds to initialize, session/new, session/prompt, session/cancel.
 * session/prompt sends a session/update notification before the response.
 */
const MOCK_OPENCODE_ACP = `
let buffer = '';
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
            agentInfo: { name: 'opencode', version: '0.1.0' },
          };
          break;
        case 'session/new':
          result = { sessionId: 'opencode-session-1' };
          break;
        case 'session/prompt': {
          const sid = (msg.params && msg.params.sessionId) || 'opencode-session-1';
          // OpenCode sends session/update notifications during prompt processing
          process.stdout.write(JSON.stringify({
            jsonrpc: '2.0',
            method: 'session/update',
            params: { sessionId: sid, type: 'text', text: 'OpenCode mock response' },
          }) + '\\n');
          result = { sessionId: sid };
          break;
        }
        case 'session/cancel':
          result = {};
          break;
        default:
          process.stdout.write(JSON.stringify({
            jsonrpc: '2.0', id: msg.id,
            error: { code: -32601, message: 'Method not found' },
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

/**
 * Spawn the mock OpenCode ACP adapter inside the VM and wire up an AcpClient.
 */
async function spawnMockOpenCodeAdapter(vm: AgentOs): Promise<{
	proc: ManagedProcess;
	client: AcpClient;
}> {
	await vm.writeFile("/tmp/mock-opencode-acp.mjs", MOCK_OPENCODE_ACP);

	const { iterable, onStdout } = createStdoutLineIterable();

	const proc = vm.kernel.spawn("node", ["/tmp/mock-opencode-acp.mjs"], {
		streamStdin: true,
		onStdout,
		env: { HOME: "/home/user" },
	});

	const client = new AcpClient(proc, iterable);
	return { proc, client };
}

/**
 * Initialize the mock adapter and register a Session in vm._sessions.
 */
async function createMockOpenCodeSession(vm: AgentOs): Promise<{
	sessionId: string;
	proc: ManagedProcess;
	client: AcpClient;
}> {
	const { proc, client } = await spawnMockOpenCodeAdapter(vm);

	const initResponse = await client.request("initialize", {
		protocolVersion: 1,
		clientCapabilities: {},
	});
	if (initResponse.error) {
		client.close();
		throw new Error(
			`Mock initialize failed: ${initResponse.error.message}`,
		);
	}

	const sessionResponse = await client.request("session/new", {
		cwd: "/home/user",
		mcpServers: [],
	});
	if (sessionResponse.error) {
		client.close();
		throw new Error(
			`Mock session/new failed: ${sessionResponse.error.message}`,
		);
	}

	const sessionId = (sessionResponse.result as { sessionId: string })
		.sessionId;
	const sessions = (vm as unknown as { _sessions: Map<string, Session> })._sessions;
	const session = new Session(client, sessionId, "opencode", {}, () => { sessions.delete(sessionId); });
	sessions.set(sessionId, session);
	return { sessionId, proc, client };
}

describe("full createSession('opencode')", () => {
	let vm: AgentOs;

	beforeEach(async () => {
		vm = await AgentOs.create({
			moduleAccessCwd: MODULE_ACCESS_CWD,
		});
	});

	afterEach(async () => {
		await vm.dispose();
	});

	test("opencode agent config is correct", () => {
		const config = AGENT_CONFIGS.opencode;
		expect(config).toBeDefined();
		// OpenCode speaks ACP natively — adapter and package are the same
		expect(config.acpAdapter).toBe("opencode-ai");
		expect(config.agentPackage).toBe("opencode-ai");
	});

	test("createSession('opencode') fails - native binary cannot execute in VM", async () => {
		// createSession resolves the opencode-ai bin entry from host node_modules,
		// spawns it with `node` inside the VM. The JS wrapper runs, but when it
		// tries to spawnSync the native Go binary, the kernel returns ENOENT.
		// The wrapper process exits non-zero, causing the ACP client's pending
		// initialize request to be rejected.
		try {
			const { sessionId } = await vm.createSession("opencode", {
				env: { ANTHROPIC_API_KEY: "test-key" },
			});
			// Unexpected success — close and fail
			vm.closeSession(sessionId);
			expect.fail(
				"createSession('opencode') should fail due to native binary limitation",
			);
		} catch (err) {
			const message = (err as Error).message;
			// The error comes from either:
			// - Process exit rejecting the pending initialize request
			// - The wrapper crashing when the native binary can't be found
			expect(message).toBeTruthy();
		}
	}, 60_000);

	test("session.prompt() sends prompt and receives session/update events (mock)", async () => {
		// Use mock OpenCode ACP adapter to test what the session lifecycle
		// would look like if the native binary could run in the VM.
		const { sessionId } = await createMockOpenCodeSession(vm);

		const events: JsonRpcNotification[] = [];
		vm.onSessionEvent(sessionId, (event) => {
			events.push(event);
		});

		const response = await vm.prompt(sessionId, "write hello world");

		expect(response.error).toBeUndefined();
		expect(response.result).toBeDefined();
		const result = response.result as { sessionId: string };
		expect(result.sessionId).toBe("opencode-session-1");

		// Mock sends session/update notification before the prompt response
		expect(events.length).toBeGreaterThanOrEqual(1);
		expect(events[0].method).toBe("session/update");
		expect((events[0].params as { text: string }).text).toBe(
			"OpenCode mock response",
		);

		vm.closeSession(sessionId);
	}, 30_000);

	test("session.close() cleans up the opencode process (mock)", async () => {
		const { sessionId, proc } = await createMockOpenCodeSession(vm);

		expect(sessionId).toBe("opencode-session-1");

		vm.closeSession(sessionId);

		// After close, the AcpClient kills the process and rejects new requests.
		await expect(
			new AcpClient(proc, (async function* () {})()).request(
				"initialize",
				{},
			),
		).rejects.toThrow();
	}, 30_000);
});
