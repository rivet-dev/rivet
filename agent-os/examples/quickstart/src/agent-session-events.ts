// Subscribe to session events and auto-approve permissions.
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

// Subscribe to session update events (streaming agent output).
session.onSessionEvent((event) => {
	const params = event.params as Record<string, unknown> | undefined;
	if (params?.type === "text_delta") {
		process.stdout.write(params.delta as string);
	} else {
		console.log("[event]", event.method, params?.type ?? "");
	}
});

// Auto-approve all permission requests.
session.onPermissionRequest((request) => {
	console.log("[permission]", request.description ?? request.permissionId);
	session.respondPermission(request.permissionId, "once");
});

// Send a prompt. Events stream in while we await the final response.
const response = await session.prompt(
	"Write a file /tmp/hello.txt containing 'hello world', then read it back.",
);
console.log("\n\nFinal response:", JSON.stringify(response.result, null, 2));

// Check event history.
const events = session.getEvents();
console.log(`\nTotal events received: ${events.length}`);

// Brief delay for async permission response to complete.
await new Promise((r) => setTimeout(r, 100));

session.close();
await vm.dispose();
await stopMockLlm(mock);
