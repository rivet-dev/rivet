// Create an agent session and send a prompt.
//
// Uses llmock as the LLM backend and a mock ACP adapter because PI CLI can't
// complete startup in the VM yet (ESM module linking limitation). When fixed:
//   const session = await vm.createSession("pi", { env: { ANTHROPIC_BASE_URL: llmUrl } });

import { AgentOs } from "@rivet-dev/agent-os-core";
import {
	createMockSession,
	startMockLlm,
	stopMockLlm,
} from "./mock-acp-adapter.js";

// Start llmock on the host with a text fixture.
const { url: ANTHROPIC_BASE_URL, port, mock } = await startMockLlm();

const vm = await AgentOs.create({ loopbackExemptPorts: [port] });

const session = await createMockSession(vm, ANTHROPIC_BASE_URL);

console.log("Session ID:", session.sessionId);
console.log("Agent type:", session.agentType);
console.log("Capabilities:", session.capabilities);

// Send a prompt and get the response.
const response = await session.prompt(
	"What is 2 + 2? Reply with just the number.",
);
console.log("Response:", JSON.stringify(response.result, null, 2));

session.close();
await vm.dispose();
await stopMockLlm(mock);
