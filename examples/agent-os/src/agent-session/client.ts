// Create an agent session and send a prompt using the PI coding agent.
//
// NOTE: This example requires ANTHROPIC_API_KEY and a working PI agent
// runtime. It may not complete in all environments.

import { createClient } from "rivetkit/client";
import type { registry } from "./server.ts";

const client = createClient<typeof registry>("http://localhost:6420");
const agent = client.vm.getOrCreate(["my-agent"]);

const ANTHROPIC_API_KEY = process.env.ANTHROPIC_API_KEY;
if (!ANTHROPIC_API_KEY) {
	console.error("ANTHROPIC_API_KEY is required.");
	process.exit(1);
}

// Create a session with the PI coding agent
const session = (await agent.createSession("pi", {
	env: { ANTHROPIC_API_KEY },
})) as { sessionId: string };
console.log("Session ID:", session.sessionId);

// Send a prompt and wait for the response
const response = await agent.sendPrompt(
	session.sessionId,
	"What is 2 + 2? Reply with just the number.",
);
console.log("Response:", JSON.stringify(response));

// Close the session
await agent.closeSession(session.sessionId);
