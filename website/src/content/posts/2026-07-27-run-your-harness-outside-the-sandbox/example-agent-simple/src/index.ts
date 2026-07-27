import * as readline from "node:readline/promises";
import { anthropic } from "@ai-sdk/anthropic";
import { docker } from "@computesdk/docker";
import { generateText, type ModelMessage, stepCountIs, tool } from "ai";
import { z } from "zod";

// Boot a sandbox on the local Docker provider. Swap `docker` for any other
// provider (e2b, daytona, vercel, etc.).
const compute = docker({
	runtime: "node",
	image: { name: "node:22-slim", pullPolicy: "ifNotPresent" },
	// Recent Docker engines dropped support for the provider's default API version
	connection: { version: "v1.44" },
});
const sandbox = await compute.sandbox.create();

// The agent's tools run here in your own process. The sandbox only receives
// the commands and file operations that the tools send to it.
const tools = {
	runCommand: tool({
		description: "Run a shell command in the sandbox",
		inputSchema: z.object({
			command: z.string().describe("The shell command to run"),
		}),
		execute: async ({ command }) => {
			console.log(`$ ${command}`);
			const result = await sandbox.runCommand(command);
			return {
				stdout: result.stdout,
				stderr: result.stderr,
				exitCode: result.exitCode,
			};
		},
	}),
	writeFile: tool({
		description: "Write a file in the sandbox",
		inputSchema: z.object({
			path: z.string().describe("Absolute path of the file"),
			content: z.string().describe("Full contents of the file"),
		}),
		execute: async ({ path, content }) => {
			console.log(`write ${path}`);
			await sandbox.filesystem.writeFile(path, content);
			return { ok: true };
		},
	}),
	readFile: tool({
		description: "Read a file from the sandbox",
		inputSchema: z.object({
			path: z.string().describe("Absolute path of the file"),
		}),
		execute: async ({ path }) => {
			console.log(`read ${path}`);
			return { content: await sandbox.filesystem.readFile(path) };
		},
	}),
};

// Read tasks from the terminal and run the agent on each one
const terminal = readline.createInterface({
	input: process.stdin,
	output: process.stdout,
});

// The session history
const messages: ModelMessage[] = [];

while (true) {
	// Wait for the user to type a task
	const prompt = await terminal.question("agent> ");
	messages.push({ role: "user", content: prompt });

	// Run the agent loop until the task is complete
	const result = await generateText({
		model: anthropic("claude-sonnet-5"),
		system:
			"You are a coding agent working inside a fresh Linux sandbox with Node.js installed. " +
			"Use the tools to run commands and edit files to complete the user's task.",
		messages,
		tools,
		stopWhen: stepCountIs(20),
	});
	messages.push(...result.response.messages);

	console.log(`\n${result.text}\n`);
}
