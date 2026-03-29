// Create an agent session with MCP servers attached.
//
// Demonstrates MCP server configuration types. Uses llmock as the LLM backend
// and a mock adapter because PI CLI can't complete startup in the VM yet.
// When fixed:
//   const session = await vm.createSession("pi", { env: { ANTHROPIC_BASE_URL: llmUrl }, mcpServers });

import type { McpServerConfig } from "@rivet-dev/agent-os-core";
import { AgentOs } from "@rivet-dev/agent-os-core";
import {
	createMockSession,
	startMockLlm,
	stopMockLlm,
} from "./mock-acp-adapter.js";

// Start llmock on the host with a text fixture.
const { url: ANTHROPIC_BASE_URL, port, mock } = await startMockLlm();

const vm = await AgentOs.create({ loopbackExemptPorts: [port] });

// MCP server configurations. These would be passed to createSession().
// Local servers are spawned as child processes inside the VM.
// Remote servers connect via URL.
const _mcpServers: McpServerConfig[] = [
	{
		type: "local",
		command: "node",
		args: ["/path/to/mcp-server.js"],
		env: { LOG_LEVEL: "info" },
	},
	// {
	//   type: "remote",
	//   url: "https://mcp.example.com/v1",
	//   headers: { Authorization: "Bearer ..." },
	// },
];

// Using mock session (real createSession would pass mcpServers).
const session = await createMockSession(vm, ANTHROPIC_BASE_URL);

console.log("Session created with MCP servers");
console.log("Agent:", session.agentInfo);
console.log("Capabilities:", session.capabilities);

const response = await session.prompt("List available tools from MCP servers.");
console.log("Response:", JSON.stringify(response.result, null, 2));

session.close();
await vm.dispose();
await stopMockLlm(mock);
