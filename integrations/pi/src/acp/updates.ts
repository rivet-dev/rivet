import type {
	ContentBlock,
	SessionUpdate,
	ToolCallContent,
	ToolKind,
} from "@agentclientprotocol/sdk";
import type { AgentSession, AgentSessionEvent } from "@earendil-works/pi-coding-agent";

type PiMessage = AgentSession["messages"][number];

/** A Pi prompt built from ACP content blocks. */
export interface PiPromptInput {
	text: string;
	images: { type: "image"; data: string; mimeType: string }[];
}

/** Converts an ACP prompt into Pi's text and images. Links and embedded files become text. */
export function toPiPrompt(blocks: ContentBlock[]): PiPromptInput {
	const text: string[] = [];
	const images: PiPromptInput["images"] = [];
	for (const block of blocks) {
		switch (block.type) {
			case "text":
				text.push(block.text);
				break;
			case "image":
				images.push({ type: "image", data: block.data, mimeType: block.mimeType });
				break;
			case "resource_link":
				text.push(block.uri);
				break;
			case "resource":
				text.push(
					"text" in block.resource
						? `${block.resource.uri}\n${block.resource.text}`
						: block.resource.uri,
				);
				break;
			case "audio":
				break;
		}
	}
	return { text: text.join("\n\n"), images };
}

function toolKind(name: string): ToolKind {
	switch (name) {
		case "read":
			return "read";
		case "edit":
		case "write":
			return "edit";
		case "bash":
			return "execute";
		case "grep":
		case "find":
		case "ls":
			return "search";
		default:
			return "other";
	}
}

function toolTitle(name: string, args: unknown): string {
	const record = (args ?? {}) as { command?: unknown; path?: unknown; pattern?: unknown };
	if (name === "bash" && typeof record.command === "string") return record.command;
	if (typeof record.path === "string") return `${name} ${record.path}`;
	if (typeof record.pattern === "string") return `${name} ${record.pattern}`;
	return name;
}

function resultText(result: unknown): string {
	const content = (result as { content?: unknown } | undefined)?.content;
	if (!Array.isArray(content)) return "";
	return content
		.filter((part): part is { type: "text"; text: string } => part?.type === "text")
		.map((part) => part.text)
		.join("\n");
}

/**
 * The diff Zed shows for an edit. Pi's `edit` replaces text fragments and
 * `write` replaces the whole file, so both map to ACP diffs of those texts.
 */
function editDiffs(name: string, args: unknown): ToolCallContent[] | undefined {
	const record = (args ?? {}) as {
		path?: unknown;
		content?: unknown;
		edits?: { oldText?: unknown; newText?: unknown }[];
		oldText?: unknown;
		newText?: unknown;
	};
	if (typeof record.path !== "string") return undefined;
	const path = record.path;
	if (name === "write" && typeof record.content === "string") {
		return [{ type: "diff", path, oldText: null, newText: record.content }];
	}
	if (name !== "edit") return undefined;
	const edits = record.edits ?? [{ oldText: record.oldText, newText: record.newText }];
	const diffs = edits.flatMap((edit): ToolCallContent[] =>
		typeof edit.oldText === "string" && typeof edit.newText === "string"
			? [{ type: "diff", path, oldText: edit.oldText, newText: edit.newText }]
			: [],
	);
	return diffs.length > 0 ? diffs : undefined;
}

function textContent(text: string): ToolCallContent[] | undefined {
	return text ? [{ type: "content", content: { type: "text", text } }] : undefined;
}

/**
 * Turns Pi session events into ACP session updates. It remembers which tool
 * calls the client has seen, so each call is announced once and then updated.
 */
export class PiEventTranslator {
	/** Tool calls the client has seen, with their latest arguments. */
	readonly #calls = new Map<string, { name: string; args: unknown }>();

	translate(event: AgentSessionEvent): SessionUpdate[] {
		switch (event.type) {
			case "message_update":
				return this.#messageUpdate(event.assistantMessageEvent);
			case "tool_execution_start":
				return [this.#toolCall(event.toolCallId, event.toolName, event.args, "in_progress")];
			case "tool_execution_update":
				return [
					{
						sessionUpdate: "tool_call_update",
						toolCallId: event.toolCallId,
						status: "in_progress",
						content: textContent(resultText(event.partialResult)),
					},
				];
			case "tool_execution_end":
				return [this.#toolEnd(event.toolCallId, event.result, event.isError)];
			case "auto_retry_start":
				return [notice(`Model request failed, retrying (${event.attempt}/${event.maxAttempts}).`)];
			case "compaction_start":
				return [notice("Compacting the conversation history.")];
			case "compaction_end":
				return event.aborted || event.errorMessage
					? [notice("Compaction did not finish.")]
					: [notice("Compacted the conversation history.")];
			case "agent_start":
			case "agent_end":
			case "agent_settled":
			case "turn_start":
			case "turn_end":
			case "message_start":
			case "message_end":
			case "queue_update":
			case "entry_appended":
			case "session_info_changed":
			case "thinking_level_changed":
			case "auto_retry_end":
			case "summarization_retry_scheduled":
			case "summarization_retry_attempt_start":
			case "summarization_retry_finished":
			case "bash_execution_update":
				return [];
			default:
				return event satisfies never;
		}
	}

	#messageUpdate(
		update: Extract<AgentSessionEvent, { type: "message_update" }>["assistantMessageEvent"],
	): SessionUpdate[] {
		switch (update.type) {
			case "text_delta":
				return [{ sessionUpdate: "agent_message_chunk", content: { type: "text", text: update.delta } }];
			case "thinking_delta":
				return [{ sessionUpdate: "agent_thought_chunk", content: { type: "text", text: update.delta } }];
			case "toolcall_start":
			case "toolcall_end": {
				const call =
					update.type === "toolcall_end" ? update.toolCall : update.partial.content[update.contentIndex];
				if (call?.type !== "toolCall") return [];
				return [this.#toolCall(call.id, call.name, call.arguments, "pending")];
			}
			case "start":
			case "text_start":
			case "text_end":
			case "thinking_start":
			case "thinking_end":
			case "toolcall_delta":
			case "done":
			case "error":
				return [];
			default:
				return update satisfies never;
		}
	}

	#toolCall(
		toolCallId: string,
		name: string,
		args: unknown,
		status: "pending" | "in_progress",
	): SessionUpdate {
		const fields = { toolCallId, title: toolTitle(name, args), kind: toolKind(name), status, rawInput: args };
		const seen = this.#calls.has(toolCallId);
		this.#calls.set(toolCallId, { name, args });
		return seen ? { sessionUpdate: "tool_call_update", ...fields } : { sessionUpdate: "tool_call", ...fields };
	}

	#toolEnd(toolCallId: string, result: unknown, isError: boolean): SessionUpdate {
		const call = this.#calls.get(toolCallId);
		this.#calls.delete(toolCallId);
		return {
			sessionUpdate: "tool_call_update",
			toolCallId,
			status: isError ? "failed" : "completed",
			content:
				(call && !isError ? editDiffs(call.name, call.args) : undefined) ??
				textContent(resultText(result)),
			rawOutput: result,
		};
	}
}

export function notice(text: string): SessionUpdate {
	return { sessionUpdate: "agent_message_chunk", content: { type: "text", text: `\n\n_${text}_\n\n` } };
}

/** Replays a stored conversation as ACP updates for `session/load`. */
export function replayMessages(messages: readonly PiMessage[]): SessionUpdate[] {
	const updates: SessionUpdate[] = [];
	const calls = new Map<string, { name: string; args: unknown }>();
	for (const message of messages) {
		switch (message.role) {
			case "user": {
				const text =
					typeof message.content === "string" ? message.content : resultText({ content: message.content });
				if (text) updates.push({ sessionUpdate: "user_message_chunk", content: { type: "text", text } });
				break;
			}
			case "assistant":
				for (const part of message.content) {
					if (part.type === "text" && part.text) {
						updates.push({ sessionUpdate: "agent_message_chunk", content: { type: "text", text: part.text } });
					} else if (part.type === "thinking" && part.thinking) {
						updates.push({
							sessionUpdate: "agent_thought_chunk",
							content: { type: "text", text: part.thinking },
						});
					} else if (part.type === "toolCall") {
						calls.set(part.id, { name: part.name, args: part.arguments });
						updates.push({
							sessionUpdate: "tool_call",
							toolCallId: part.id,
							title: toolTitle(part.name, part.arguments),
							kind: toolKind(part.name),
							status: "in_progress",
							rawInput: part.arguments,
						});
					}
				}
				break;
			case "toolResult": {
				const call = calls.get(message.toolCallId);
				updates.push({
					sessionUpdate: "tool_call_update",
					toolCallId: message.toolCallId,
					status: message.isError ? "failed" : "completed",
					content:
						(call && !message.isError ? editDiffs(call.name, call.args) : undefined) ??
						textContent(resultText(message)),
				});
				break;
			}
			case "system":
			case "custom":
			case "branchSummary":
			case "bashExecution":
			case "compactionSummary":
				break;
			default:
				message satisfies never;
		}
	}
	return updates;
}
