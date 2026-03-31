// Host toolkits are exposed as CLI commands inside the VM.
//
// When toolkits are registered in server.ts, CLI shims are auto-installed
// at /usr/local/bin/agentos-{toolkit} and the tool list is injected into
// the agent's system prompt.
//
// The agent calls tools as shell commands:
//   agentos-weather forecast --city Paris --days 3
//
// Responses are JSON:
//   {"ok":true,"result":{"city":"Paris","days":3,"temperature":22,"conditions":"sunny"}}

import { createClient } from "rivetkit/client";
import type { registry } from "./server.ts";

const client = createClient<typeof registry>("http://localhost:6420");
const agent = client.vm.getOrCreate(["my-agent"]);

// Call a tool using the auto-generated CLI command
const result = (await agent.exec("agentos-weather forecast --city Paris --days 3")) as {
	stdout: string;
	exitCode: number;
};
console.log("Weather:", result.stdout.trim());

// Create a session and let the agent use tools naturally
const session = (await agent.createSession("pi", {
	env: { ANTHROPIC_API_KEY: process.env.ANTHROPIC_API_KEY! },
})) as { sessionId: string };

// The agent will call agentos-weather forecast automatically
await agent.sendPrompt(session.sessionId, "What's the weather in Paris?");
