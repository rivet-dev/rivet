import { openai } from "@ai-sdk/openai";
import { createMCPClient } from "@ai-sdk/mcp";
import { streamText, tool, type CoreMessage } from "ai";
import { z } from "zod";
import { actor, event, queue } from "rivetkit";
import { db } from "rivetkit/db";

// Shared types for client usage
export type ChatMessage = {
	id: string;
	role: "user" | "assistant";
	content: string;
	createdAt: number;
};

export type ResponseEvent = {
	messageId: string;
	delta: string;
	content: string;
	done: boolean;
	error?: string;
};

export type CodeUpdateEvent = {
	code: string;
	revision: number;
};

export type CodeAgentState = {
	messages: ChatMessage[];
	code: string;
	codeRevision: number;
	status: "idle" | "thinking" | "error";
	hasApiKey: boolean;
};

export const DEFAULT_ACTOR_CODE = `import { actor, event } from "rivetkit";

export default actor({
	state: {
		count: 0,
	},
	events: {
		countChanged: event(),
	},
	actions: {
		increment: (c, amount = 1) => {
			c.state.count += amount;
			c.broadcast("countChanged", c.state.count);
			return c.state.count;
		},
		decrement: (c, amount = 1) => {
			c.state.count -= amount;
			c.broadcast("countChanged", c.state.count);
			return c.state.count;
		},
		getCount: (c) => c.state.count,
		reset: (c) => {
			c.state.count = 0;
			c.broadcast("countChanged", 0);
			return 0;
		},
	},
});
`;

const SYSTEM_PROMPT = `You are an AI assistant that helps users build RivetKit actors. When the user asks you to create or modify actor code, use the updateCode tool to set the code.

The module MUST use \`export default actor({...})\` syntax with imports from "rivetkit". State must be JSON-serializable. Actions return JSON-serializable values.

CRITICAL: The generated code runs as plain JavaScript (.mjs), NOT TypeScript. Do NOT use TypeScript syntax like generics (event<number>()), type annotations (x: string), "as" casts, or interfaces. Use plain JavaScript only.

Always provide the FULL actor code via the updateCode tool, not partial changes. You can explain what you changed in your text response.

You have access to the official RivetKit documentation via MCP tools (docs.search, docs.get, docs.list). When generating or modifying actor code, ALWAYS search the docs first to ensure you use the correct APIs. For example, search for "actor state", "actor events", "actor actions", "schedule", "kv", "db", or whatever features the user is asking about. Use docs.get to fetch the full content of a specific doc page when you need more detail.`;

const buildId = (prefix: string) =>
	`${prefix}-${Date.now()}-${Math.random().toString(16).slice(2)}`;

async function getConfig(
	database: any,
	key: string,
	defaultValue: string,
): Promise<string> {
	const rows = await database.execute<{ value: string }>(
		"SELECT value FROM config WHERE key = ?",
		key,
	);
	return rows[0]?.value ?? defaultValue;
}

async function setConfig(
	database: any,
	key: string,
	value: string,
): Promise<void> {
	await database.execute(
		"INSERT OR REPLACE INTO config (key, value) VALUES (?, ?)",
		key,
		value,
	);
}

export const codeAgent = actor({
	state: {},
	db: db({
		onMigrate: async (database) => {
			await database.execute(`
				CREATE TABLE IF NOT EXISTS messages (
					id TEXT PRIMARY KEY,
					role TEXT NOT NULL,
					content TEXT NOT NULL DEFAULT '',
					created_at INTEGER NOT NULL
				);
				CREATE TABLE IF NOT EXISTS config (
					key TEXT PRIMARY KEY,
					value TEXT NOT NULL
				);
			`);
		},
	}),
	queues: {
		chat: queue<{ text: string; currentCode?: string; reasoning?: string }>(),
	},
	events: {
		response: event<ResponseEvent>(),
		codeUpdated: event<CodeUpdateEvent>(),
		statusChanged: event<string>(),
	},

	run: async (c: any) => {
		console.log("[codeAgent] run loop started");
		for await (const queued of c.queue.iter()) {
			const { body } = queued;
			console.log("[codeAgent] received queue message");
			if (!body?.text || typeof body.text !== "string") {
				continue;
			}

			const userMessageId = buildId("user");
			await c.db.execute(
				"INSERT INTO messages (id, role, content, created_at) VALUES (?, ?, ?, ?)",
				userMessageId,
				"user",
				body.text.trim(),
				Date.now(),
			);

			const assistantMessageId = buildId("assistant");
			await c.db.execute(
				"INSERT INTO messages (id, role, content, created_at) VALUES (?, ?, ?, ?)",
				assistantMessageId,
				"assistant",
				"",
				Date.now(),
			);

			// Notify frontend about the new assistant message placeholder.
			c.broadcast("response", {
				messageId: assistantMessageId,
				delta: "",
				content: "",
				done: false,
			});

			await setConfig(c.db, "status", "thinking");
			c.broadcast("statusChanged", "thinking");

			const history = await c.db.execute<{
				role: string;
				content: string;
			}>(
				"SELECT role, content FROM messages WHERE content != '' ORDER BY created_at",
			);

			const promptMessages: CoreMessage[] = [
				{ role: "system", content: SYSTEM_PROMPT },
			];

			if (body.currentCode) {
				promptMessages.push({
					role: "system",
					content: `The user's current actor code is:\n\`\`\`javascript\n${body.currentCode}\n\`\`\`\nModify this code according to the user's request using the updateCode tool.`,
				});
			}

			promptMessages.push(
				...history.map((m) => ({
					role: m.role as "user" | "assistant",
					content: m.content,
				})),
			);

			let content = "";
			let mcpClient: Awaited<ReturnType<typeof createMCPClient>> | null = null;

			try {
				// Connect to MCP docs server. If it fails, proceed without docs tools.
				let docsTools: Record<string, any> = {};
				try {
					mcpClient = await createMCPClient({
						transport: {
							type: "sse",
							url: "https://mcp.rivet.dev/mcp",
						},
					});
					docsTools = await mcpClient.tools();
					console.log("[codeAgent] MCP docs tools loaded:", Object.keys(docsTools));
				} catch (mcpError) {
					console.warn("[codeAgent] MCP docs unavailable, proceeding without:", mcpError);
				}

				const reasoningLevel = body.reasoning || "none";
				const providerOptions =
					reasoningLevel !== "none"
						? { openai: { reasoningEffort: reasoningLevel === "extra_high" ? "high" : reasoningLevel as "medium" | "high" } }
						: undefined;

				console.log("[codeAgent] calling streamText with gpt-4o, reasoning:", reasoningLevel);
				const result = streamText({
					model: openai("gpt-4o"),
					messages: promptMessages,
					providerOptions,
					tools: {
						...docsTools,
						updateCode: tool({
							description:
								"Update the actor code in the editor. Always provide the complete actor module source code.",
							parameters: z.object({
								code: z
									.string()
									.describe(
										"The complete actor module source code",
									),
							}),
							execute: async ({ code }) => {
								console.log(
									"[codeAgent] updateCode tool called, code length:",
									code.length,
								);
								await setConfig(c.db, "code", code);
								const revStr = await getConfig(
									c.db,
									"code_revision",
									"1",
								);
								const newRev = Number(revStr) + 1;
								await setConfig(
									c.db,
									"code_revision",
									String(newRev),
								);
								c.broadcast("codeUpdated", {
									code,
									revision: newRev,
								});
								return { success: true, revision: newRev };
							},
						}),
					},
					maxSteps: 5,
				});

				for await (const part of result.fullStream) {
					if (c.aborted) break;

					if (part.type === "text-delta") {
						content += part.textDelta;
						c.broadcast("response", {
							messageId: assistantMessageId,
							delta: part.textDelta,
							content,
							done: false,
						});
					}
				}

				await c.db.execute(
					"UPDATE messages SET content = ? WHERE id = ?",
					content,
					assistantMessageId,
				);

				c.broadcast("response", {
					messageId: assistantMessageId,
					delta: "",
					content,
					done: true,
				});

				await setConfig(c.db, "status", "idle");
				c.broadcast("statusChanged", "idle");
			} catch (error) {
				console.error("code agent error:", error);

				const errorMessage =
					error instanceof Error ? error.message : "Unknown error";

				const finalContent =
					content ||
					"Something went wrong while generating a response.";

				await c.db.execute(
					"UPDATE messages SET content = ? WHERE id = ?",
					finalContent,
					assistantMessageId,
				);

				await setConfig(c.db, "status", "error");

				c.broadcast("response", {
					messageId: assistantMessageId,
					delta: "",
					content: finalContent,
					done: true,
					error: errorMessage,
				});
				c.broadcast("statusChanged", "error");
			} finally {
				await mcpClient?.close();
			}
		}
	},

	actions: {
		setCode: async (c: any, newCode: string) => {
			await setConfig(c.db, "code", newCode);
			const revStr = await getConfig(c.db, "code_revision", "1");
			const newRev = Number(revStr) + 1;
			await setConfig(c.db, "code_revision", String(newRev));
			c.broadcast("codeUpdated", { code: newCode, revision: newRev });
			return { revision: newRev };
		},
		getState: async (c: any): Promise<CodeAgentState> => {
			const messages = await c.db.execute<ChatMessage>(
				"SELECT id, role, content, created_at as createdAt FROM messages ORDER BY created_at",
			);
			const code = await getConfig(c.db, "code", DEFAULT_ACTOR_CODE);
			const codeRevision = Number(
				await getConfig(c.db, "code_revision", "1"),
			);
			const status = (await getConfig(c.db, "status", "idle")) as
				| "idle"
				| "thinking"
				| "error";

			return {
				messages,
				code,
				codeRevision,
				status,
				hasApiKey: !!process.env.OPENAI_API_KEY,
			};
		},
	},
});
