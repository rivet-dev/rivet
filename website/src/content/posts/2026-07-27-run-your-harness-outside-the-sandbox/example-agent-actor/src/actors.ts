import { anthropic } from "@ai-sdk/anthropic";
import { docker } from "@computesdk/docker";
import { generateText, type ModelMessage, stepCountIs, tool } from "ai";
import { actor, queue, setup } from "rivetkit";
import { db } from "rivetkit/db";
import { z } from "zod";

// Use the local Docker provider. Swap `docker` for any other provider
// (e2b, daytona, vercel, etc.).
const compute = docker({
	runtime: "node",
	image: { name: "node:22-slim", pullPolicy: "ifNotPresent" },
	// Recent Docker engines dropped support for the provider's default API version
	connection: { version: "v1.44" },
});

type Sandbox = Awaited<ReturnType<typeof compute.sandbox.create>>;

interface Vars {
	sandbox: Sandbox;
	tools: ReturnType<typeof createTools>;
}

export const codingAgent = actor({
	// The sandbox id is persisted so the actor can reattach to the same
	// sandbox across sleeps and restarts
	state: { sandboxId: null as string | null },

	// Attach the sandbox when the actor wakes. Vars hold non-serializable
	// handles like the sandbox connection.
	createVars: async (c): Promise<Vars> => {
		// Reattach to this session's sandbox, or boot a new one on first wake
		let sandbox = c.state.sandboxId
			? await compute.sandbox.getById(c.state.sandboxId)
			: null;
		if (!sandbox) {
			sandbox = await compute.sandbox.create();
			c.state.sandboxId = sandbox.sandboxId;
		}

		return { sandbox, tools: createTools(sandbox) };
	},

	options: {
		// Booting the sandbox takes longer than the default
		createVarsTimeout: 60_000,
	},

	// Session history lives in the actor's own SQLite database
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS messages (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					message TEXT NOT NULL
				);
			`);
		},
	}),

	// Tasks arrive on a durable queue instead of an action
	queues: {
		prompt: queue<{ content: string }, { text: string }>(),
	},

	// Process one task at a time off the queue
	run: async (c) => {
		for await (const message of c.queue.iter({ completable: true })) {
			// Load the session history from SQLite
			const rows = (await c.db.execute(
				"SELECT message FROM messages ORDER BY id",
			)) as { message: string }[];
			const history = rows.map(
				(row) => JSON.parse(row.message) as ModelMessage,
			);

			// Run the agent loop until the task is complete
			const userMessage: ModelMessage = {
				role: "user",
				content: message.body.content,
			};
			const result = await generateText({
				model: anthropic("claude-sonnet-5"),
				system:
					"You are a coding agent working inside a fresh Linux sandbox with Node.js installed. " +
					"Use the tools to run commands and edit files to complete the user's task.",
				messages: [...history, userMessage],
				tools: c.vars.tools,
				stopWhen: stepCountIs(20),
			});

			// Append the new user message and everything the agent produced
			for (const newMessage of [userMessage, ...result.response.messages]) {
				await c.db.execute(
					"INSERT INTO messages (message) VALUES (?)",
					JSON.stringify(newMessage),
				);
			}

			// Reply to the caller waiting on this task
			await message.complete({ text: result.text });
		}
	},
});

export const registry = setup({ use: { codingAgent } });

// The agent's tools run in the actor. The sandbox only receives the
// commands and file operations that the tools send to it.
function createTools(sandbox: Sandbox) {
	return {
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
}
