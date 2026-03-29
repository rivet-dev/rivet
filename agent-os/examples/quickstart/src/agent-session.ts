// Create an agent session and send a prompt using the PI coding agent.
//
// NOTE: This example requires ANTHROPIC_API_KEY and a working PI agent
// runtime. It may not complete in all environments.

import { AgentOs } from "@rivet-dev/agent-os-core";
import common from "@rivet-dev/agent-os-common";
import pi from "@rivet-dev/agent-os-pi";

const ANTHROPIC_API_KEY = process.env.ANTHROPIC_API_KEY;
if (!ANTHROPIC_API_KEY) {
	console.error("ANTHROPIC_API_KEY is required.");
	process.exit(1);
}

const vm = await AgentOs.create({ software: [common, pi] });

// Create a session with the PI coding agent
const { sessionId } = await vm.createSession("pi", {
	env: { ANTHROPIC_API_KEY },
});
console.log("Session ID:", sessionId);

// Send a prompt and wait for the response
const response = await vm.prompt(sessionId, "What is 2 + 2? Reply with just the number.");
console.log("Response:", JSON.stringify(response.result, null, 2));

// Close the session
vm.closeSession(sessionId);
await vm.dispose();
