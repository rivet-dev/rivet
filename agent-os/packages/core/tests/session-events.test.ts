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
 * Mock ACP adapter that sends multiple session/update notifications per prompt.
 * Sends 3 notifications: status, text, then final text before responding.
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
            agentInfo: { name: 'events-agent', version: '1.0.0' },
            agentCapabilities: {},
          };
          break;

        case 'session/new':
          sessionCounter++;
          result = { sessionId: 'evt-session-' + sessionCounter };
          break;

        case 'session/prompt': {
          const sid = (msg.params && msg.params.sessionId) || 'unknown';
          // Send 3 session/update notifications in order
          process.stdout.write(JSON.stringify({
            jsonrpc: '2.0',
            method: 'session/update',
            params: { sessionId: sid, type: 'status', text: 'Thinking...' },
          }) + '\\n');
          process.stdout.write(JSON.stringify({
            jsonrpc: '2.0',
            method: 'session/update',
            params: { sessionId: sid, type: 'text', text: 'Part 1' },
          }) + '\\n');
          process.stdout.write(JSON.stringify({
            jsonrpc: '2.0',
            method: 'session/update',
            params: { sessionId: sid, type: 'text', text: 'Part 2' },
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

describe("session event history", () => {
	let vm: AgentOs;

	beforeEach(async () => {
		vm = await AgentOs.create();
	});

	afterEach(async () => {
		await vm.dispose();
	});

	test("getEvents() returns empty array before any prompts", async () => {
		const { sessionId } = await createTrackedSession(vm, "/tmp/evt-1.mjs");

		expect(vm.getSessionEvents(sessionId).length).toBe(0);

		vm.closeSession(sessionId);
	}, 30_000);

	test("getEvents() returns notifications accumulated during prompt()", async () => {
		const { sessionId } = await createTrackedSession(vm, "/tmp/evt-2.mjs");

		await vm.prompt(sessionId, "trigger events");

		const events = vm.getSessionEvents(sessionId).map((e) => e.notification);
		// Mock sends 3 session/update notifications per prompt
		expect(events.length).toBeGreaterThanOrEqual(3);
		expect(events[0].method).toBe("session/update");

		vm.closeSession(sessionId);
	}, 30_000);

	test("event ordering matches notification arrival order", async () => {
		const { sessionId } = await createTrackedSession(vm, "/tmp/evt-3.mjs");

		await vm.prompt(sessionId, "check order");

		const events = vm
			.getSessionEvents(sessionId, { method: "session/update" })
			.map((e) => e.notification);
		expect(events.length).toBeGreaterThanOrEqual(3);

		// Extract the text values in order (VM stdout can duplicate lines)
		const texts = events.map((e) => (e.params as { text: string }).text);
		// The three expected values must appear in order
		const thinkIdx = texts.indexOf("Thinking...");
		const part1Idx = texts.indexOf("Part 1");
		const part2Idx = texts.indexOf("Part 2");
		expect(thinkIdx).toBeGreaterThanOrEqual(0);
		expect(part1Idx).toBeGreaterThan(thinkIdx);
		expect(part2Idx).toBeGreaterThan(part1Idx);

		vm.closeSession(sessionId);
	}, 30_000);

	test("events have sequential sequence numbers", async () => {
		const { sessionId } = await createTrackedSession(vm, "/tmp/evt-4.mjs");

		await vm.prompt(sessionId, "sequence check");

		const sequenced = vm.getSessionEvents(sessionId);
		expect(sequenced.length).toBeGreaterThanOrEqual(3);

		// Sequence numbers are monotonically increasing
		for (let i = 1; i < sequenced.length; i++) {
			expect(sequenced[i].sequenceNumber).toBeGreaterThan(
				sequenced[i - 1].sequenceNumber,
			);
		}
		// First sequence number is 0
		expect(sequenced[0].sequenceNumber).toBe(0);

		vm.closeSession(sessionId);
	}, 30_000);

	test("getEvents({ since: N }) filters to events after sequence N", async () => {
		const { sessionId } = await createTrackedSession(vm, "/tmp/evt-5.mjs");

		await vm.prompt(sessionId, "filter test");

		const all = vm.getSessionEvents(sessionId);
		expect(all.length).toBeGreaterThanOrEqual(3);

		// Filter: only events after the first one
		const afterFirst = vm
			.getSessionEvents(sessionId, { since: all[0].sequenceNumber })
			.map((e) => e.notification);
		expect(afterFirst.length).toBe(all.length - 1);

		// Filter: only events after the second one
		const afterSecond = vm
			.getSessionEvents(sessionId, { since: all[1].sequenceNumber })
			.map((e) => e.notification);
		expect(afterSecond.length).toBe(all.length - 2);

		vm.closeSession(sessionId);
	}, 30_000);

	test("getEvents({ method: 'session/update' }) filters by method", async () => {
		const { sessionId } = await createTrackedSession(vm, "/tmp/evt-6.mjs");

		await vm.prompt(sessionId, "method filter");

		const allEvents = vm.getSessionEvents(sessionId).map((e) => e.notification);
		const updateEvents = vm
			.getSessionEvents(sessionId, { method: "session/update" })
			.map((e) => e.notification);

		// All events from this mock are session/update
		expect(updateEvents.length).toBe(allEvents.length);

		// Filtering by a non-existent method returns empty
		const noEvents = vm
			.getSessionEvents(sessionId, { method: "nonexistent/method" })
			.map((e) => e.notification);
		expect(noEvents.length).toBe(0);

		vm.closeSession(sessionId);
	}, 30_000);

	test("event history persists across multiple prompt() calls", async () => {
		const { sessionId } = await createTrackedSession(vm, "/tmp/evt-7.mjs");

		await vm.prompt(sessionId, "first prompt");
		const afterFirst = vm.getSessionEvents(sessionId).length;
		expect(afterFirst).toBeGreaterThanOrEqual(3);

		await vm.prompt(sessionId, "second prompt");
		const afterSecond = vm.getSessionEvents(sessionId).length;
		// Second prompt adds more events — history accumulates
		expect(afterSecond).toBeGreaterThan(afterFirst);

		// Sequence numbers continue from where they left off
		const sequenced = vm.getSessionEvents(sessionId);
		const lastFromFirst = sequenced[afterFirst - 1].sequenceNumber;
		const firstFromSecond = sequenced[afterFirst].sequenceNumber;
		expect(firstFromSecond).toBeGreaterThan(lastFromFirst);

		vm.closeSession(sessionId);
	}, 30_000);

	test("event history cleared after session close", async () => {
		const { sessionId } = await createTrackedSession(vm, "/tmp/evt-8.mjs");

		await vm.prompt(sessionId, "before close");
		expect(vm.getSessionEvents(sessionId).length).toBeGreaterThanOrEqual(3);

		vm.closeSession(sessionId);

		// After close, session is removed from tracking
		expect(() => vm.getSessionEvents(sessionId)).toThrow("Session not found");
	}, 30_000);
});
