import { streamText, tool, stepCountIs, type Tool, type CoreMessage } from "ai";
import { anthropic } from "@ai-sdk/anthropic";
import { experimental_createMCPClient as createMCPClient } from "@ai-sdk/mcp";
import { StreamableHTTPClientTransport } from "@modelcontextprotocol/sdk/client/streamableHttp.js";
import { z } from "zod";
import { SYSTEM_MESSAGE } from "./system";
import { FreestyleDevServerFilesystem } from "freestyle-sandboxes";
import OpenAI from "openai";
import type { UIMessage } from "../shared/types";

const CLAUDE_SONNET_MODEL = "claude-sonnet-4-20250514";

export interface AIServiceOptions {
	maxSteps?: number;
	maxOutputTokens?: number;
	abortSignal?: AbortSignal;
	onStepUpdate?: (text: string) => void;
	onFinish?: () => void;
	onError?: (error: { error: unknown }) => void;
}

export interface AIResponse {
	text: string;
	messages: unknown[];
}

// Lazily create the OpenAI client for Morph API
let morphClient: OpenAI | null = null;
function getMorphClient(): OpenAI {
	if (!morphClient) {
		morphClient = new OpenAI({
			apiKey: process.env.MORPH_API_KEY,
			baseURL: "https://api.morphllm.com/v1",
		});
	}
	return morphClient;
}

// Create the todo tool using AI SDK v5 format
const todoTool = tool({
	description:
		"Use the update todo list tool to keep track of the tasks you need to do to accomplish the user's request. You should update the todo list each time you complete an item. You can remove tasks from the todo list, but only if they are no longer relevant or you've finished the user's request completely and they are asking for something else. Make sure to update the todo list each time the user asks you do something new. If they're asking for something new, you should probably just clear the whole todo list and start over with new items. For complex logic, use multiple todos to ensure you get it all right rather than just a single todo for implementing all logic.",
	inputSchema: z.object({
		items: z.array(
			z.object({
				description: z.string(),
				completed: z.boolean(),
			})
		),
	}),
	execute: async () => {
		return {};
	},
});

// Create the morph edit tool using AI SDK v5 format
function createMorphTool(fs: FreestyleDevServerFilesystem) {
	return tool({
		description:
			"Use this tool to make an edit to an existing file.\n\nThis will be read by a less intelligent model, which will quickly apply the edit. You should make it clear what the edit is, while also minimizing the unchanged code you write.\nWhen writing the edit, you should specify each edit in sequence, with the special comment // ... existing code ... to represent unchanged code in between edited lines.\n\nFor example:\n\n// ... existing code ...\nFIRST_EDIT\n// ... existing code ...\nSECOND_EDIT\n// ... existing code ...\nTHIRD_EDIT\n// ... existing code ...\n\nYou should still bias towards repeating as few lines of the original file as possible to convey the change.\nBut, each edit should contain sufficient context of unchanged lines around the code you're editing to resolve ambiguity.\nDO NOT omit spans of pre-existing code (or comments) without using the // ... existing code ... comment to indicate its absence. If you omit the existing code comment, the model may inadvertently delete these lines.\nIf you plan on deleting a section, you must provide context before and after to delete it. If the initial code is ```code \\n Block 1 \\n Block 2 \\n Block 3 \\n code```, and you want to remove Block 2, you would output ```// ... existing code ... \\n Block 1 \\n  Block 3 \\n // ... existing code ...```.\nMake sure it is clear what the edit should be, and where it should be applied.\nMake edits to a file in a single edit_file call instead of multiple edit_file calls to the same file. The apply model can handle many distinct edits at once.",
		inputSchema: z.object({
			target_file: z.string().describe("The target file to modify."),
			instructions: z
				.string()
				.describe(
					"A single sentence instruction describing what you are going to do for the sketched edit. This is used to assist the less intelligent model in applying the edit. Use the first person to describe what you are going to do. Use it to disambiguate uncertainty in the edit."
				),
			code_edit: z
				.string()
				.describe(
					"Specify ONLY the precise lines of code that you wish to edit. NEVER specify or write out unchanged code. Instead, represent all unchanged code using the comment of the language you're editing in - example: // ... existing code ..."
				),
		}),
		execute: async ({ target_file, instructions, code_edit }) => {
			let file;
			try {
				file = await fs.readFile(target_file);
			} catch (error) {
				throw new Error(
					`File not found: ${target_file}. Error message: ${error instanceof Error ? error.message : String(error)}`
				);
			}
			const response = await getMorphClient().chat.completions.create({
				model: "morph-v3-large",
				messages: [
					{
						role: "user",
						content: `<instruction>${instructions}</instruction>\n<code>${file}</code>\n<update>${code_edit}</update>`,
					},
				],
			});

			const finalCode = response.choices[0].message.content;

			if (!finalCode) {
				throw new Error("No code returned from Morph API.");
			}
			await fs.writeFile(target_file, finalCode);
			return { success: true };
		},
	});
}

// Convert UIMessage to the format expected by generateText
function convertUIMessageToMessages(message: UIMessage): CoreMessage[] {
	const content = message.parts
		.map((part) => {
			if (part.type === "text") {
				return part.text;
			}
			return "";
		})
		.join("");

	return [{ role: message.role as "user" | "assistant", content }];
}

/**
 * Send a message to the AI and get a response using vanilla AI SDK
 */
export async function sendMessage(
	_appId: string,
	mcpUrl: string,
	fs: FreestyleDevServerFilesystem,
	message: UIMessage,
	previousMessages: CoreMessage[],
	options?: AIServiceOptions
): Promise<AIResponse> {
	console.log("[sendMessage] Starting...", { mcpUrl });

	// Create MCP client for Freestyle dev server tools using Streamable HTTP transport
	console.log("[sendMessage] Creating MCP client with Streamable HTTP transport...");
	const httpTransport = new StreamableHTTPClientTransport(new URL(mcpUrl));
	const mcpClient = await createMCPClient({
		transport: httpTransport,
	});
	console.log("[sendMessage] MCP client created");

	try {
		// Get MCP tools from the Freestyle dev server
		console.log("[sendMessage] Getting MCP tools...");
		const mcpTools = await mcpClient.tools();
		console.log("[sendMessage] MCP tools retrieved", { toolCount: Object.keys(mcpTools).length });

		// Build the tools object - use type assertion to handle the MCP tools
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		const tools: Record<string, Tool<any, any>> = {
			update_todo_list: todoTool,
			...mcpTools,
		};

		// Add morph tool if API key is available
		if (process.env.MORPH_API_KEY) {
			tools.edit_file = createMorphTool(fs);
			console.log("[sendMessage] Morph edit_file tool added");
		}

		// Convert the incoming message
		const newMessages = convertUIMessageToMessages(message);
		console.log("[sendMessage] Message converted", { messageCount: newMessages.length });

		// Combine previous messages with new message
		const allMessages: CoreMessage[] = [...previousMessages, ...newMessages];
		console.log("[sendMessage] All messages combined", { totalMessages: allMessages.length });

		// Call streamText with the AI SDK
		console.log("[sendMessage] Calling streamText...");
		let accumulatedText = "";
		const result = streamText({
			model: anthropic(CLAUDE_SONNET_MODEL),
			system: SYSTEM_MESSAGE,
			messages: allMessages,
			tools,
			stopWhen: stepCountIs(options?.maxSteps ?? 100),
			maxOutputTokens: options?.maxOutputTokens ?? 64000,
			abortSignal: options?.abortSignal,
			onStepFinish: (step) => {
				console.log("[sendMessage] Step finished", {
					textLength: step.text?.length || 0,
					toolCallCount: step.toolCalls?.length || 0,
					toolResultCount: step.toolResults?.length || 0,
				});

				// Add tool calls to the accumulated text so users can see agent activity
				if (step.toolCalls && step.toolCalls.length > 0) {
					for (const tc of step.toolCalls) {
						// eslint-disable-next-line @typescript-eslint/no-explicit-any
						const args = (tc as any).input ?? (tc as any).args;
						if (accumulatedText) accumulatedText += "\n\n";
						accumulatedText += `**Tool Call: ${tc.toolName}**\n\`\`\`json\n${JSON.stringify(args, null, 2)}\n\`\`\``;
					}
					if (options?.onStepUpdate) {
						options.onStepUpdate(accumulatedText);
					}
				}

				// Add tool results to the accumulated text
				if (step.toolResults && step.toolResults.length > 0) {
					for (const tr of step.toolResults) {
						// eslint-disable-next-line @typescript-eslint/no-explicit-any
						const result = (tr as any).output ?? (tr as any).result;
						if (accumulatedText) accumulatedText += "\n\n";
						// Truncate large results for readability
						const resultStr = JSON.stringify(result, null, 2);
						const truncatedResult = resultStr.length > 500
							? resultStr.substring(0, 500) + "\n... (truncated)"
							: resultStr;
						accumulatedText += `**Tool Result: ${tr.toolName}**\n\`\`\`json\n${truncatedResult}\n\`\`\``;
					}
					if (options?.onStepUpdate) {
						options.onStepUpdate(accumulatedText);
					}
				}

				// Add newline separator between steps if there was text
				if (step.text && accumulatedText) {
					accumulatedText += "\n\n";
				}
			},
		});

		// Stream text tokens in real-time
		for await (const chunk of result.textStream) {
			accumulatedText += chunk;
			if (options?.onStepUpdate) {
				options.onStepUpdate(accumulatedText);
			}
		}

		// Get final response after stream completes
		const finalResponse = await result.response;
		console.log("[sendMessage] streamText completed", { textLength: accumulatedText?.length || 0 });

		options?.onFinish?.();

		console.log("[sendMessage] Returning result");
		// Return accumulated text (all steps combined) instead of result.text (only last step)
		return {
			text: accumulatedText,
			messages: finalResponse?.messages || [],
		};
	} catch (error) {
		console.error("[sendMessage] Error occurred:", error);
		options?.onError?.({ error });
		throw error;
	} finally {
		// Always close the MCP client
		console.log("[sendMessage] Closing MCP client...");
		await mcpClient.close();
		console.log("[sendMessage] MCP client closed");
	}
}
