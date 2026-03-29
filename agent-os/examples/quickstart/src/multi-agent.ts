// Run multiple agent sessions in the same VM with a shared filesystem.
//
// Both sessions are created concurrently with Promise.all.
// Uses llmock as the LLM backend and a mock ACP adapter because PI CLI can't
// complete startup in the VM yet (ESM module linking limitation). When fixed:
//   const [agent1, agent2] = await Promise.all([
//     vm.createSession("pi", { env: { ANTHROPIC_BASE_URL: llmUrl } }),
//     vm.createSession("pi", { env: { ANTHROPIC_BASE_URL: llmUrl } }),
//   ]);

import { AgentOs } from "@rivet-dev/agent-os-core";
import {
	createMockSession,
	startMockLlm,
	stopMockLlm,
} from "./mock-acp-adapter.js";

// Start llmock on the host with a text fixture.
const { url: ANTHROPIC_BASE_URL, port, mock } = await startMockLlm();

const vm = await AgentOs.create({ loopbackExemptPorts: [port] });

// Create a shared workspace.
await vm.mkdir("/workspace");
await vm.writeFile(
	"/workspace/spec.md",
	"# Spec\nBuild a hello world CLI tool.",
);

// Create both agent sessions concurrently.
const [agent1, agent2] = await Promise.all([
	createMockSession(vm, ANTHROPIC_BASE_URL),
	createMockSession(vm, ANTHROPIC_BASE_URL),
]);
console.log("Agent 1:", agent1.sessionId);
console.log("Agent 2:", agent2.sessionId);

// Agent 1: reads the spec and "writes" code.
const r1 = await agent1.prompt(
	"Read /workspace/spec.md and write /workspace/hello.mjs that prints 'Hello World'.",
);
console.log("Agent 1 response:", (r1.result as Record<string, unknown>).result);

// Simulate agent 1's work by writing the file it would have created.
await vm.writeFile("/workspace/hello.mjs", 'console.log("Hello World");');

// Agent 2: sees what Agent 1 wrote (shared filesystem).
const r2 = await agent2.prompt(
	"Read /workspace/hello.mjs and describe what it does.",
);
console.log("Agent 2 response:", (r2.result as Record<string, unknown>).result);

// Verify the shared filesystem worked.
const content = await vm.readFile("/workspace/hello.mjs");
console.log("\nShared file contents:", new TextDecoder().decode(content));

agent1.close();
agent2.close();
await vm.dispose();
await stopMockLlm(mock);
