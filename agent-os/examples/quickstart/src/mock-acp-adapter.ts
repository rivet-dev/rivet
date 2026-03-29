// Mock ACP adapter for quickstart examples.
//
// Uses @copilotkit/llmock for realistic LLM responses instead of hardcoded
// strings. The ACP protocol layer is still mocked because PI CLI can't
// complete startup in the VM yet (ESM module linking limitation).
// Once that's fixed, replace createMockSession() with:
//   const session = await vm.createSession("pi", { env: { ANTHROPIC_BASE_URL: llmUrl } });

import type { Fixture } from "@copilotkit/llmock";
import { LLMock } from "@copilotkit/llmock";
import {
	AcpClient,
	type AgentOs,
	Session,
	createStdoutLineIterable,
} from "@rivet-dev/agent-os-core";

/** Default fixture that matches any prompt and returns a simple text response. */
export const DEFAULT_FIXTURE: Fixture = {
	match: { predicate: () => true },
	response: { content: "Hello from llmock" },
};

/**
 * Start a mock LLM server on the host using llmock.
 * Returns the base URL, port, and mock instance for cleanup.
 */
export async function startMockLlm(
	fixtures?: Fixture[],
): Promise<{ url: string; port: number; mock: LLMock }> {
	const mock = new LLMock({ port: 0, logLevel: "silent" });
	mock.addFixtures(fixtures ?? [DEFAULT_FIXTURE]);
	const url = await mock.start();
	const port = Number(new URL(url).port);
	return { url, port, mock };
}

/**
 * Stop a running mock LLM server.
 */
export async function stopMockLlm(mock: LLMock): Promise<void> {
	await mock.stop();
}

/**
 * Generate the mock ACP adapter script that runs inside the VM.
 * Handles the ACP JSON-RPC protocol and calls llmock for prompt responses.
 */
function generateMockScript(llmBaseUrl: string): string {
	return `
let buffer = '';
let sessionCounter = 0;

process.stdin.resume();
process.stdin.on('data', (chunk) => {
  const str = chunk instanceof Uint8Array ? new TextDecoder().decode(chunk) : String(chunk);
  buffer += str;
  processBuffer();
});

async function processBuffer() {
  while (true) {
    const idx = buffer.indexOf('\\n');
    if (idx === -1) break;
    const line = buffer.substring(0, idx);
    buffer = buffer.substring(idx + 1);
    if (!line.trim()) continue;

    try {
      const msg = JSON.parse(line);
      if (msg.id === undefined) continue;
      await handleMessage(msg);
    } catch (e) {}
  }
}

async function handleMessage(msg) {
  let result;
  switch (msg.method) {
    case 'initialize':
      result = {
        protocolVersion: 1,
        agentInfo: { name: 'mock-agent', version: '1.0.0' },
        agentCapabilities: {
          permissions: true,
          plan_mode: true,
          tool_calls: true,
          text_messages: true,
          session_lifecycle: true,
          streaming_deltas: true,
        },
        modes: {
          currentModeId: 'normal',
          availableModes: [
            { id: 'normal', label: 'Normal' },
            { id: 'plan', label: 'Plan' },
          ],
        },
      };
      break;
    case 'session/new':
      sessionCounter++;
      result = { sessionId: 'mock-session-' + sessionCounter };
      break;
    case 'session/prompt': {
      const sid = (msg.params && msg.params.sessionId) || 'mock-session-1';
      const promptText = msg.params?.prompt?.[0]?.text || '';

      // Call llmock Anthropic Messages API for a realistic response.
      let responseText;
      try {
        const resp = await fetch('${llmBaseUrl}/v1/messages', {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
            'x-api-key': 'mock-key',
            'anthropic-version': '2023-06-01',
          },
          body: JSON.stringify({
            model: 'claude-sonnet-4-20250514',
            max_tokens: 1024,
            messages: [{ role: 'user', content: promptText }],
          }),
        });
        const data = await resp.json();
        responseText = data.content?.[0]?.text ?? 'No response from LLM';
      } catch (e) {
        responseText = 'LLM request failed: ' + (e.message || e);
      }

      // Stream text_delta events.
      for (const char of responseText) {
        process.stdout.write(JSON.stringify({
          jsonrpc: '2.0',
          method: 'session/update',
          params: { sessionId: sid, type: 'text_delta', delta: char },
        }) + '\\n');
      }

      // Final text event.
      process.stdout.write(JSON.stringify({
        jsonrpc: '2.0',
        method: 'session/update',
        params: { sessionId: sid, type: 'text', text: responseText },
      }) + '\\n');

      // Simulate permission request if prompt mentions files.
      if (/file|write/i.test(promptText)) {
        process.stdout.write(JSON.stringify({
          jsonrpc: '2.0',
          method: 'request/permission',
          params: {
            sessionId: sid,
            permissionId: 'perm-' + Date.now(),
            description: 'Write to /tmp/hello.txt',
          },
        }) + '\\n');
      }

      result = {
        sessionId: sid,
        result: [{ type: 'text', text: responseText }],
      };
      break;
    }
    case 'session/cancel':
      result = { sessionId: msg.params?.sessionId };
      break;
    case 'request/permission':
      result = { ok: true };
      break;
    default:
      result = {};
  }

  process.stdout.write(JSON.stringify({
    jsonrpc: '2.0',
    id: msg.id,
    result,
  }) + '\\n');
}
`;
}

/**
 * Create a mock agent session inside the VM.
 * Uses llmock for realistic LLM responses via the Anthropic Messages API.
 *
 * The VM must be created with loopbackExemptPorts including the llmock port
 * so the mock script can reach the host-side llmock server.
 */
export async function createMockSession(
	vm: AgentOs,
	llmBaseUrl: string,
): Promise<Session> {
	const scriptPath = `/tmp/_mock-acp-${Date.now()}-${Math.random().toString(36).slice(2)}.js`;
	await vm.writeFile(scriptPath, generateMockScript(llmBaseUrl));

	const { iterable, onStdout } = createStdoutLineIterable();

	const proc = vm.spawn("node", [scriptPath], {
		streamStdin: true,
		onStdout,
	});

	const client = new AcpClient(proc, iterable);

	const initResp = await client.request("initialize", {
		protocolVersion: 1,
		clientCapabilities: {},
	});
	if (initResp.error) {
		client.close();
		throw new Error(`Initialize failed: ${initResp.error.message}`);
	}

	const sessionResp = await client.request("session/new", {
		cwd: "/home/user",
		mcpServers: [],
	});
	if (sessionResp.error) {
		client.close();
		throw new Error(`session/new failed: ${sessionResp.error.message}`);
	}

	const sessionId = (sessionResp.result as { sessionId: string }).sessionId;
	const initResult = initResp.result as Record<string, unknown>;

	return new Session(client, sessionId, "mock", {
		capabilities: initResult.agentCapabilities as Record<string, boolean>,
		agentInfo: initResult.agentInfo as { name: string; version?: string },
		modes: initResult.modes as {
			currentModeId: string;
			availableModes: Array<{ id: string; label?: string }>;
		},
	});
}
