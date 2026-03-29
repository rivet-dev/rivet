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
 * Comprehensive mock ACP adapter that supports all protocol methods and returns
 * rich initialize data (capabilities, modes, configOptions, agentInfo).
 * session/prompt sends a session/update notification before the response.
 * If prompt text contains "permission", sends a request/permission notification.
 * Each session/new call returns a unique sessionId.
 */
const COMPREHENSIVE_MOCK = `
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
            agentInfo: { name: 'comprehensive-agent', version: '1.0.0' },
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
            modes: {
              currentModeId: 'normal',
              availableModes: [
                { id: 'normal', label: 'Normal' },
                { id: 'plan', label: 'Plan' },
              ],
            },
            configOptions: [
              { id: 'model-opt', category: 'model', label: 'Model', currentValue: 'default', allowedValues: [{ id: 'default' }, { id: 'opus' }] },
              { id: 'thought-opt', category: 'thought_level', label: 'Thought Level', currentValue: 'medium' },
            ],
          };
          break;

        case 'session/new': {
          sessionCounter++;
          const mcpServers = (msg.params && msg.params.mcpServers) || [];
          result = { sessionId: 'comp-session-' + sessionCounter, mcpServers };
          break;
        }

        case 'session/prompt': {
          const sid = (msg.params && msg.params.sessionId) || 'unknown';
          const promptText = (msg.params && msg.params.prompt && msg.params.prompt[0] && msg.params.prompt[0].text) || '';

          if (promptText.includes('permission')) {
            process.stdout.write(JSON.stringify({
              jsonrpc: '2.0',
              method: 'request/permission',
              params: {
                sessionId: sid,
                permissionId: 'perm-test-001',
                description: 'Execute command: test',
                tool: 'shell',
              },
            }) + '\\n');
          }

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

        case 'session/set_mode':
          result = { modeId: msg.params.modeId, applied: true };
          break;

        case 'session/set_config_option':
          result = { configId: msg.params.configId, value: msg.params.value, applied: true };
          break;

        case 'request/permission':
          result = { acknowledged: true };
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

let globalSessionCounter = 0;

/**
 * Spawn a mock adapter in the VM, initialize ACP, create a session,
 * and register it in AgentOs._sessions with an onClose callback.
 * Returns the sessionId, proc, and client for test use.
 */
async function createTrackedSession(
	vm: AgentOs,
	scriptPath: string,
): Promise<{
	sessionId: string;
	proc: ManagedProcess;
	client: AcpClient;
}> {
	// Inject a unique prefix so each adapter process generates unique session IDs
	const prefix = `s${++globalSessionCounter}`;
	const script = COMPREHENSIVE_MOCK.replace(
		"'comp-session-' + sessionCounter",
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

	// Wire up onClose to remove from VM's tracking
	const sessions = (vm as unknown as { _sessions: Map<string, Session> })
		._sessions;
	const session = new Session(client, sessionId, "mock", initData, () => {
		sessions.delete(sessionId);
	});
	sessions.set(sessionId, session);

	return { sessionId, proc, client };
}

describe("comprehensive session API tests", () => {
	let vm: AgentOs;

	beforeEach(async () => {
		vm = await AgentOs.create();
	});

	afterEach(async () => {
		await vm.dispose();
	});

	test("permission request flow -- mock agent sends permission request, test calls respondPermission, agent continues", async () => {
		const { sessionId } = await createTrackedSession(
			vm,
			"/tmp/perm-mock.mjs",
		);

		const permissionRequests: {
			permissionId: string;
			description?: string;
		}[] = [];
		vm.onPermissionRequest(sessionId, (req) => {
			permissionRequests.push(req);
		});

		// Prompt with "permission" triggers the mock to emit request/permission
		const response = await vm.prompt(sessionId, "test permission flow");

		expect(response.error).toBeUndefined();
		// VM stdout can duplicate lines; check at least 1 permission request arrived
		expect(permissionRequests.length).toBeGreaterThanOrEqual(1);
		expect(permissionRequests[0].permissionId).toBe("perm-test-001");

		// Respond to the permission request
		const permResp = await vm.respondPermission(
			sessionId,
			"perm-test-001",
			"once",
		);
		expect(permResp.error).toBeUndefined();
		expect(
			(permResp.result as { acknowledged: boolean }).acknowledged,
		).toBe(true);

		vm.closeSession(sessionId);
	}, 30_000);

	test("setMode changes session mode", async () => {
		const { sessionId } = await createTrackedSession(
			vm,
			"/tmp/mode-mock.mjs",
		);

		const response = await vm.setSessionMode(sessionId, "plan");
		expect(response.error).toBeUndefined();
		const result = response.result as { modeId: string; applied: boolean };
		expect(result.modeId).toBe("plan");
		expect(result.applied).toBe(true);

		vm.closeSession(sessionId);
	}, 30_000);

	test("setModel changes model configuration", async () => {
		const { sessionId } = await createTrackedSession(
			vm,
			"/tmp/model-mock.mjs",
		);

		const response = await vm.setSessionModel(sessionId, "opus");
		expect(response.error).toBeUndefined();
		const result = response.result as {
			configId: string;
			value: string;
			applied: boolean;
		};
		expect(result.configId).toBe("model-opt");
		expect(result.value).toBe("opus");
		expect(result.applied).toBe(true);

		vm.closeSession(sessionId);
	}, 30_000);

	test("getConfigOptions returns available options", async () => {
		const { sessionId } = await createTrackedSession(
			vm,
			"/tmp/config-mock.mjs",
		);

		const options = vm.getSessionConfigOptions(sessionId);
		expect(options.length).toBe(2);
		expect(options[0].category).toBe("model");
		expect(options[0].id).toBe("model-opt");
		expect(options[1].category).toBe("thought_level");

		vm.closeSession(sessionId);
	}, 30_000);

	test("multiple concurrent sessions on same VM work independently", async () => {
		// Create both sessions — each goes through initialize + session/new
		const { sessionId: sessionId1 } = await createTrackedSession(
			vm,
			"/tmp/multi-mock-1.mjs",
		);
		const { sessionId: sessionId2 } = await createTrackedSession(
			vm,
			"/tmp/multi-mock-2.mjs",
		);

		// Sessions have different IDs (each adapter process has its own counter)
		expect(sessionId1).not.toBe(sessionId2);

		// Both are tracked by the VM
		expect(vm.listSessions().length).toBe(2);

		// Both have independent capabilities
		expect(vm.getSessionCapabilities(sessionId1)!.permissions).toBe(true);
		expect(vm.getSessionCapabilities(sessionId2)!.permissions).toBe(true);

		// Both have independent agent info
		expect(vm.getSessionAgentInfo(sessionId1)).toEqual(
			vm.getSessionAgentInfo(sessionId2),
		);

		// Closing one doesn't affect the other
		vm.closeSession(sessionId1);
		expect(vm.listSessions().length).toBe(1);
		expect(
			vm.listSessions().find((s) => s.sessionId === sessionId2) !== undefined,
		).toBe(true);

		// Second session still works
		const resp = await vm.prompt(sessionId2, "independent prompt");
		expect(resp.error).toBeUndefined();

		vm.closeSession(sessionId2);
		expect(vm.listSessions().length).toBe(0);
	}, 30_000);

	test("listSessions returns all active sessions", async () => {
		const { sessionId: sessionId1 } = await createTrackedSession(
			vm,
			"/tmp/list-mock-1.mjs",
		);
		const { sessionId: sessionId2 } = await createTrackedSession(
			vm,
			"/tmp/list-mock-2.mjs",
		);

		const list = vm.listSessions();
		expect(list.length).toBe(2);

		const ids = list.map((s) => s.sessionId);
		expect(ids).toContain(sessionId1);
		expect(ids).toContain(sessionId2);

		vm.closeSession(sessionId1);
		vm.closeSession(sessionId2);
	}, 30_000);

	test("resumeSession retrieves session by ID", async () => {
		const { sessionId } = await createTrackedSession(
			vm,
			"/tmp/getsession-mock.mjs",
		);

		const retrieved = vm.resumeSession(sessionId);
		expect(retrieved.sessionId).toBe(sessionId);

		// Not-found throws
		expect(() => vm.resumeSession("nonexistent-id")).toThrow(
			"Session not found",
		);

		vm.closeSession(sessionId);
	}, 30_000);

	test("closeSession removes session from VM tracking", async () => {
		const { sessionId } = await createTrackedSession(
			vm,
			"/tmp/close-track-mock.mjs",
		);

		expect(vm.listSessions().length).toBe(1);

		vm.closeSession(sessionId);

		expect(vm.listSessions().length).toBe(0);
		expect(() => vm.resumeSession(sessionId)).toThrow(
			"Session not found",
		);
	}, 30_000);

	test("dispose with multiple active sessions closes all", async () => {
		const { sessionId: sessionId1 } = await createTrackedSession(
			vm,
			"/tmp/dispose-mock-1.mjs",
		);
		const { sessionId: sessionId2 } = await createTrackedSession(
			vm,
			"/tmp/dispose-mock-2.mjs",
		);

		expect(vm.listSessions().length).toBe(2);

		await vm.dispose();

		// After dispose, both sessions are removed from tracking
		expect(vm.listSessions().length).toBe(0);

		// Re-create vm so afterEach's dispose() doesn't double-dispose
		vm = await AgentOs.create();
	}, 30_000);

	test("prompt after close throws", async () => {
		const { sessionId } = await createTrackedSession(
			vm,
			"/tmp/closed-prompt-mock.mjs",
		);

		vm.closeSession(sessionId);

		await expect(vm.prompt(sessionId, "should fail")).rejects.toThrow(
			"Session not found",
		);
	}, 30_000);

	test("createSession with mcpServers passes config through to agent", async () => {
		// Use manual adapter spawn to verify mcpServers are sent in session/new
		await vm.writeFile("/tmp/mcp-mock.mjs", COMPREHENSIVE_MOCK);
		const { iterable, onStdout } = createStdoutLineIterable();
		const proc = vm.kernel.spawn("node", ["/tmp/mcp-mock.mjs"], {
			streamStdin: true,
			onStdout,
			env: { HOME: "/home/user" },
		});
		const client = new AcpClient(proc, iterable);

		await client.request("initialize", {
			protocolVersion: 1,
			clientCapabilities: {},
		});

		const mcpServers = [
			{ type: "remote", url: "https://mcp.example.com", headers: {} },
		];

		const sessionResp = await client.request("session/new", {
			cwd: "/home/user",
			mcpServers,
		});

		expect(sessionResp.error).toBeUndefined();
		const result = sessionResp.result as {
			sessionId: string;
			mcpServers: unknown[];
		};
		expect(result.mcpServers).toEqual(mcpServers);

		client.close();
	}, 30_000);

	test("session capabilities accessible after createSession", async () => {
		const { sessionId } = await createTrackedSession(
			vm,
			"/tmp/caps-mock.mjs",
		);

		const caps = vm.getSessionCapabilities(sessionId)!;
		expect(caps.permissions).toBe(true);
		expect(caps.plan_mode).toBe(true);
		expect(caps.questions).toBe(false);
		expect(caps.tool_calls).toBe(true);
		expect(caps.text_messages).toBe(true);
		expect(caps.images).toBe(false);
		expect(caps.session_lifecycle).toBe(true);
		expect(caps.mcp_tools).toBe(true);

		expect(vm.getSessionAgentInfo(sessionId)).toEqual({
			name: "comprehensive-agent",
			version: "1.0.0",
		});

		vm.closeSession(sessionId);
	}, 30_000);

	test("getSessionEvents() returns accumulated events", async () => {
		const { sessionId } = await createTrackedSession(
			vm,
			"/tmp/events-mock.mjs",
		);

		// Before any prompts, no events
		expect(vm.getSessionEvents(sessionId).length).toBe(0);

		// Send a prompt -- adapter sends a session/update notification
		await vm.prompt(sessionId, "trigger events");

		const sequenced = vm.getSessionEvents(sessionId);
		expect(sequenced.length).toBeGreaterThanOrEqual(1);

		const events = sequenced.map((e) => e.notification);
		const updateEvents = events.filter(
			(e) => e.method === "session/update",
		);
		expect(updateEvents.length).toBeGreaterThanOrEqual(1);

		// Filter by method
		const filtered = vm.getSessionEvents(sessionId, { method: "session/update" });
		expect(filtered.length).toBe(updateEvents.length);

		// Sequenced events have sequence numbers
		expect(sequenced[0].sequenceNumber).toBe(0);
		if (sequenced.length > 1) {
			expect(sequenced[1].sequenceNumber).toBeGreaterThan(
				sequenced[0].sequenceNumber,
			);
		}

		// Filter by since
		const sinceLast = vm.getSessionEvents(sessionId, {
			since: sequenced[0].sequenceNumber,
		});
		expect(sinceLast.length).toBe(sequenced.length - 1);

		vm.closeSession(sessionId);
	}, 30_000);

	test("resumeSession returns existing session", async () => {
		const { sessionId } = await createTrackedSession(
			vm,
			"/tmp/resume-mock.mjs",
		);

		// resumeSession returns the same session ID
		const resumed = vm.resumeSession(sessionId);
		expect(resumed.sessionId).toBe(sessionId);

		// Resumed session is fully functional
		const response = await vm.prompt(resumed.sessionId, "after resume");
		expect(response.error).toBeUndefined();

		// Throws for unknown sessionId
		expect(() => vm.resumeSession("nonexistent")).toThrow(
			"Session not found",
		);

		vm.closeSession(sessionId);
	}, 30_000);

	test("setThoughtLevel sends config option with correct configId", async () => {
		const { sessionId } = await createTrackedSession(
			vm,
			"/tmp/thought-mock.mjs",
		);

		const response = await vm.setSessionThoughtLevel(sessionId, "high");
		expect(response.error).toBeUndefined();
		const result = response.result as {
			configId: string;
			value: string;
			applied: boolean;
		};
		// Should resolve 'thought_level' category to its config option id 'thought-opt'
		expect(result.configId).toBe("thought-opt");
		expect(result.value).toBe("high");
		expect(result.applied).toBe(true);

		vm.closeSession(sessionId);
	}, 30_000);

	test("setThoughtLevel falls back to category as configId when no matching option", async () => {
		// Create a session with no configOptions so there's no thought_level category match
		const script = COMPREHENSIVE_MOCK.replace(
			"'comp-session-' + sessionCounter",
			"'no-config-' + sessionCounter",
		).replace(/configOptions: \[[\s\S]*?\],\n/, "configOptions: [],\n");
		await vm.writeFile("/tmp/no-config-mock.mjs", script);
		const { iterable, onStdout } = createStdoutLineIterable();
		const proc = vm.kernel.spawn("node", ["/tmp/no-config-mock.mjs"], {
			streamStdin: true,
			onStdout,
			env: { HOME: "/home/user" },
		});
		const client = new AcpClient(proc, iterable);

		const initResp = await client.request("initialize", {
			protocolVersion: 1,
			clientCapabilities: {},
		});
		const sessionResp = await client.request("session/new", {
			cwd: "/home/user",
			mcpServers: [],
		});

		const initResult = initResp.result as Record<string, unknown>;
		const sessionId = (sessionResp.result as { sessionId: string })
			.sessionId;

		const initData: SessionInitData = {};
		if (initResult.agentCapabilities) {
			initData.capabilities =
				initResult.agentCapabilities as AgentCapabilities;
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

		const response = await vm.setSessionThoughtLevel(sessionId, "high");
		expect(response.error).toBeUndefined();
		const result = response.result as {
			configId: string;
			value: string;
			applied: boolean;
		};
		// Falls back to using 'thought_level' (the category name) as configId
		expect(result.configId).toBe("thought_level");
		expect(result.value).toBe("high");
		expect(result.applied).toBe(true);

		vm.closeSession(sessionId);
	}, 30_000);

	test("setThoughtLevel on closed session throws", async () => {
		const { sessionId } = await createTrackedSession(
			vm,
			"/tmp/thought-closed-mock.mjs",
		);

		vm.closeSession(sessionId);

		await expect(vm.setSessionThoughtLevel(sessionId, "high")).rejects.toThrow(
			"Session not found",
		);
	}, 30_000);

	test("destroySession tears down session gracefully", async () => {
		const { sessionId } = await createTrackedSession(
			vm,
			"/tmp/destroy-mock.mjs",
		);

		expect(vm.listSessions().length).toBe(1);

		// destroySession sends cancel then closes
		await vm.destroySession(sessionId);

		// Session is removed from tracking
		expect(vm.listSessions().length).toBe(0);
		expect(() => vm.resumeSession(sessionId)).toThrow("Session not found");

		// Session is gone — subsequent operations throw "Session not found"
		await expect(vm.prompt(sessionId, "should fail")).rejects.toThrow(
			"Session not found",
		);

		// destroySession on unknown ID throws
		await expect(vm.destroySession("nonexistent")).rejects.toThrow(
			"Session not found",
		);
	}, 30_000);

	test("getModes() returns modes from initialize, null without modes, unchanged after setMode()", async () => {
		const { sessionId } = await createTrackedSession(
			vm,
			"/tmp/getmodes-mock.mjs",
		);

		// 1. getSessionModes() returns SessionModeState from initialize response
		const modes = vm.getSessionModes(sessionId);
		expect(modes).not.toBeNull();
		expect(modes?.currentModeId).toBe("normal");
		expect(modes?.availableModes).toHaveLength(2);
		expect(modes?.availableModes[0]).toEqual({
			id: "normal",
			label: "Normal",
		});
		expect(modes?.availableModes[1]).toEqual({ id: "plan", label: "Plan" });

		// 2. getSessionModes() returns null when a session was created without modes
		// Create a second session using an adapter that returns no modes
		const noModesScript = COMPREHENSIVE_MOCK.replace(
			"'comp-session-' + sessionCounter",
			"'no-modes-' + sessionCounter",
		).replace(
			/modes: \{[\s\S]*?\},\n(\s*configOptions)/,
			"// modes intentionally omitted\n$1",
		);
		await vm.writeFile("/tmp/no-modes-mock.mjs", noModesScript);
		const { iterable: iterable2, onStdout: onStdout2 } =
			createStdoutLineIterable();
		const proc2 = vm.kernel.spawn("node", ["/tmp/no-modes-mock.mjs"], {
			streamStdin: true,
			onStdout: onStdout2,
			env: { HOME: "/home/user" },
		});
		const client2 = new AcpClient(proc2, iterable2);
		const initResp2 = await client2.request("initialize", {
			protocolVersion: 1,
			clientCapabilities: {},
		});
		const sessionResp2 = await client2.request("session/new", {
			cwd: "/home/user",
			mcpServers: [],
		});
		const initResult2 = initResp2.result as Record<string, unknown>;
		const sessionId2 = (sessionResp2.result as { sessionId: string })
			.sessionId;
		const initData2: SessionInitData = {};
		if (initResult2.agentCapabilities) {
			initData2.capabilities =
				initResult2.agentCapabilities as AgentCapabilities;
		}
		if (initResult2.agentInfo) {
			initData2.agentInfo =
				initResult2.agentInfo as SessionInitData["agentInfo"];
		}
		// modes intentionally omitted from initData2
		const sessions2 = (
			vm as unknown as { _sessions: Map<string, Session> }
		)._sessions;
		const noModesSession = new Session(
			client2,
			sessionId2,
			"mock",
			initData2,
			() => {
				sessions2.delete(sessionId2);
			},
		);
		sessions2.set(sessionId2, noModesSession);
		expect(vm.getSessionModes(sessionId2)).toBeNull();
		vm.closeSession(sessionId2);

		// 3. After setMode(), getSessionModes() still returns the original modes
		// (modes are agent-reported, not client-tracked)
		const resp = await vm.setSessionMode(sessionId, "plan");
		expect(resp.error).toBeUndefined();

		const modesAfter = vm.getSessionModes(sessionId);
		expect(modesAfter).not.toBeNull();
		expect(modesAfter?.currentModeId).toBe("normal");
		expect(modesAfter?.availableModes).toHaveLength(2);

		vm.closeSession(sessionId);
	}, 30_000);
});
