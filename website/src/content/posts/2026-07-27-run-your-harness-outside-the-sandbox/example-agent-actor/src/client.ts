import * as readline from "node:readline/promises";
import { createClient } from "rivetkit/client";
import type { registry } from "./actors.ts";

const client = createClient<typeof registry>();

// Each key is an independent agent with its own actor, sandbox, and history
const agent = client.codingAgent.getOrCreate(["demo"]);

// Read tasks from the terminal and run the agent on each one
const terminal = readline.createInterface({
	input: process.stdin,
	output: process.stdout,
});

while (true) {
	// Wait for the user to type a task
	const prompt = await terminal.question("agent> ");

	// Queue the task on the actor and wait for the agent to complete it
	const result = await agent.send(
		"prompt",
		{ content: prompt },
		{ wait: true, timeout: 300_000 },
	);
	if (!result.response) throw new Error(`Task ${result.status}`);
	console.log(`\n${result.response.text}\n`);
}
