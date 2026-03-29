import type { LLMock } from "@copilotkit/llmock";
import { afterEach, beforeEach, describe, expect, test } from "vitest";
import { AcpClient } from "../src/acp-client.js";
import { AgentOs } from "../src/agent-os.js";
import type { JsonRpcNotification } from "../src/protocol.js";
import { Session } from "../src/session.js";
import { createStdoutLineIterable } from "../src/stdout-lines.js";
import {
	createAnthropicFixture,
	startLlmock,
	stopLlmock,
} from "./helpers/llmock-helper.js";

/**
 * Mock ACP adapter that acts as an LLM agent by calling the Anthropic Messages
 * API (backed by llmock). On session/prompt it:
 *   1. Sends the user message to the LLM
 *   2. If the response contains tool_use, "executes" the tool and sends the
 *      result back to the LLM in a second request
 *   3. Emits session/update notifications for tool execution and final text
 *
 * Uses fetch() which is available in the VM via the kernel network stack.
 * Falls back to mock ACP adapter approach because createSession('pi') cannot
 * spawn the PI agent binary inside the VM (bare command PATH resolution is
 * unsupported in the kernel).
 */
const MOCK_LLM_AGENT_ADAPTER = `
const BASE_URL = process.env.ANTHROPIC_BASE_URL;
const API_KEY = process.env.ANTHROPIC_API_KEY;

let buffer = '';
process.stdin.resume();
process.stdin.on('data', (chunk) => {
  const str = chunk instanceof Uint8Array ? new TextDecoder().decode(chunk) : String(chunk);
  buffer += str;
  processLines();
});

function send(obj) {
  process.stdout.write(JSON.stringify(obj) + '\\n');
}

function processLines() {
  while (true) {
    const idx = buffer.indexOf('\\n');
    if (idx === -1) break;
    const line = buffer.substring(0, idx);
    buffer = buffer.substring(idx + 1);
    if (!line.trim()) continue;
    try {
      const msg = JSON.parse(line);
      handleMsg(msg);
    } catch (e) {}
  }
}

async function callLLM(messages) {
  const resp = await fetch(BASE_URL + '/v1/messages', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'x-api-key': API_KEY,
      'anthropic-version': '2023-06-01',
    },
    body: JSON.stringify({
      model: 'claude-sonnet-4-20250514',
      max_tokens: 1024,
      messages: messages,
    }),
  });
  return resp.json();
}

async function handleMsg(msg) {
  if (msg.id === undefined) return;

  switch (msg.method) {
    case 'initialize':
      send({
        jsonrpc: '2.0', id: msg.id,
        result: { protocolVersion: 1, agentInfo: { name: 'mock-llm-agent', version: '1.0' } },
      });
      break;

    case 'session/new':
      send({
        jsonrpc: '2.0', id: msg.id,
        result: { sessionId: 'e2e-session-1' },
      });
      break;

    case 'session/prompt': {
      const sid = (msg.params && msg.params.sessionId) || 'e2e-session-1';
      const promptParts = (msg.params && msg.params.prompt) || [];
      const userText = promptParts.map(p => p.text || '').join('') || 'hello';

      try {
        const messages = [{ role: 'user', content: userText }];
        const resp1 = await callLLM(messages);
        const toolUseBlocks = (resp1.content || []).filter(b => b.type === 'tool_use');

        if (toolUseBlocks.length > 0) {
          // Notify about tool execution
          send({
            jsonrpc: '2.0',
            method: 'session/update',
            params: { sessionId: sid, type: 'tool_use', text: 'Executing tool: ' + toolUseBlocks[0].name },
          });

          // Build second request with tool result
          messages.push({ role: 'assistant', content: resp1.content });
          messages.push({
            role: 'user',
            content: [{
              type: 'tool_result',
              tool_use_id: toolUseBlocks[0].id,
              content: 'file1.txt\\nfile2.txt',
            }],
          });

          const resp2 = await callLLM(messages);
          const finalText = (resp2.content || [])
            .filter(b => b.type === 'text')
            .map(b => b.text)
            .join('');

          // Notify with final text
          send({
            jsonrpc: '2.0',
            method: 'session/update',
            params: { sessionId: sid, type: 'text', text: finalText },
          });
        }

        send({ jsonrpc: '2.0', id: msg.id, result: { sessionId: sid } });
      } catch (err) {
        send({
          jsonrpc: '2.0', id: msg.id,
          error: { code: -32000, message: 'LLM call failed: ' + String(err) },
        });
      }
      break;
    }

    default:
      send({
        jsonrpc: '2.0', id: msg.id,
        error: { code: -32601, message: 'Method not found' },
      });
  }
}
`;

describe("end-to-end mock agent session with llmock", () => {
	let vm: AgentOs;
	let mock: LLMock;
	let mockUrl: string;
	let mockPort: number;

	beforeEach(async () => {
		// First fixture: no tool role in messages yet → return tool_use (bash)
		const toolFixture = createAnthropicFixture(
			{
				predicate: (req) =>
					!req.messages.some((m) => m.role === "tool"),
			},
			{
				toolCalls: [
					{
						name: "bash",
						arguments: JSON.stringify({ command: "ls" }),
					},
				],
			},
		);

		// Second fixture: tool role present (tool result sent) → return text
		const textFixture = createAnthropicFixture(
			{
				predicate: (req) => req.messages.some((m) => m.role === "tool"),
			},
			{ content: "Task completed: found 2 files in the directory" },
		);

		const { url, mock: m } = await startLlmock([toolFixture, textFixture]);
		mock = m;
		mockUrl = url;
		mockPort = new URL(url).port;

		vm = await AgentOs.create({
			loopbackExemptPorts: [Number(mockPort)],
		});
	});

	afterEach(async () => {
		await vm.dispose();
		await stopLlmock(mock);
	});

	test("multi-turn agent session: tool_use then text response via llmock", async () => {
		await vm.writeFile("/tmp/llm-adapter.mjs", MOCK_LLM_AGENT_ADAPTER);

		const { iterable, onStdout } = createStdoutLineIterable();
		const proc = vm.kernel.spawn(
			"node",
			["/tmp/llm-adapter.mjs"],
			{
				streamStdin: true,
				onStdout,
				env: {
					HOME: "/home/user",
					ANTHROPIC_BASE_URL: mockUrl,
					ANTHROPIC_API_KEY: "mock-key",
				},
			},
		);

		const client = new AcpClient(proc, iterable);

		// Initialize ACP protocol
		const initResp = await client.request("initialize", {
			protocolVersion: 1,
			clientCapabilities: {},
		});
		expect(initResp.error).toBeUndefined();

		// Create session
		const sessionResp = await client.request("session/new", {
			cwd: "/home/user",
			mcpServers: [],
		});
		expect(sessionResp.error).toBeUndefined();
		const sessionId = (sessionResp.result as { sessionId: string })
			.sessionId;
		expect(sessionId).toBe("e2e-session-1");

		// Register session in vm._sessions so flat API methods can find it
		const sessions = (vm as unknown as { _sessions: Map<string, Session> })._sessions;
		const session = new Session(client, sessionId, "mock", {}, () => { sessions.delete(sessionId); });
		sessions.set(sessionId, session);

		// Collect session events
		const events: JsonRpcNotification[] = [];
		vm.onSessionEvent(sessionId, (event) => {
			events.push(event);
		});

		// Send prompt - triggers multi-turn: tool_use → tool_result → text
		const response = await vm.prompt(sessionId, "run ls in the current directory");
		expect(response.error).toBeUndefined();

		// Verify llmock received at least 2 requests (multi-turn)
		const requests = mock.getRequests();
		expect(requests.length).toBeGreaterThanOrEqual(2);

		// Verify events include tool_use notification and final text
		const toolEvents = events.filter(
			(e) => (e.params as { type?: string })?.type === "tool_use",
		);
		expect(toolEvents.length).toBeGreaterThanOrEqual(1);
		expect((toolEvents[0].params as { text: string }).text).toContain(
			"bash",
		);

		const textEvents = events.filter(
			(e) => (e.params as { type?: string })?.type === "text",
		);
		expect(textEvents.length).toBeGreaterThanOrEqual(1);
		expect(
			(textEvents[textEvents.length - 1].params as { text: string }).text,
		).toContain("Task completed");

		vm.closeSession(sessionId);
	}, 60_000);
});
