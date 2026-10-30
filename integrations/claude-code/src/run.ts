import {
	type Options,
	query,
	type SDKMessage,
	type SDKResultMessage,
} from "@anthropic-ai/claude-agent-sdk";
import { z } from "zod";

const PARENT_SESSION_VARIABLES = [
	"CLAUDECODE",
	"CLAUDE_CODE_ENTRYPOINT",
	"CLAUDE_CODE_SESSION_ID",
	"CLAUDE_CODE_CHILD_SESSION",
	"CLAUDE_CODE_SESSION_ATTENDED",
	"CLAUDE_CODE_MESSAGING_SOCKET",
	"CLAUDE_CODE_MESSAGING_TOKEN",
	"CLAUDE_PID",
];

export interface ClaudeCodeRequest {
	prompt: string;
	systemPrompt: string;
	tools?: readonly string[] | undefined;
	model?: string | undefined;
	cwd?: string | undefined;
	resumeSessionId?: string | undefined;
	persistSession?: boolean | undefined;
	signal?: AbortSignal | undefined;
	claudeCodePath?: string | undefined;
}

export interface ClaudeCodeRun<Output> {
	output: Output;
	sessionId: string;
	costUsd: number;
	durationMs: number;
}

export function runClaudeCode(request: ClaudeCodeRequest): Promise<ClaudeCodeRun<string>>;
export function runClaudeCode<Output>(
	request: ClaudeCodeRequest,
	outputSchema: z.ZodType<Output>,
): Promise<ClaudeCodeRun<Output>>;
export async function runClaudeCode<Output>(
	request: ClaudeCodeRequest,
	outputSchema?: z.ZodType<Output>,
): Promise<ClaudeCodeRun<Output | string>> {
	const messages = query({ prompt: request.prompt, options: sdkOptions(request, outputSchema) });
	const result = await finalResult(messages);

	if (result.subtype !== "success") {
		throw new Error(`Claude Code run failed (${result.subtype}): ${result.errors.join("; ")}`);
	}

	return {
		output: outputSchema ? outputSchema.parse(result.structured_output) : result.result,
		sessionId: result.session_id,
		costUsd: result.total_cost_usd,
		durationMs: result.duration_ms,
	};
}

function sdkOptions(request: ClaudeCodeRequest, outputSchema: z.ZodType | undefined): Options {
	const tools = [...(request.tools ?? [])];

	return {
		systemPrompt: request.systemPrompt,
		tools,
		allowedTools: tools,
		permissionMode: "dontAsk",
		settingSources: [],
		model: request.model,
		cwd: request.cwd,
		resume: request.resumeSessionId,
		persistSession: request.persistSession ?? false,
		outputFormat: outputSchema && {
			type: "json_schema",
			schema: z.toJSONSchema(outputSchema, { target: "draft-7" }),
		},
		abortController: abortControllerFollowing(request.signal),
		env: environmentWithoutParentSession(),
		pathToClaudeCodeExecutable: request.claudeCodePath,
	};
}

async function finalResult(messages: AsyncIterable<SDKMessage>): Promise<SDKResultMessage> {
	let result: SDKResultMessage | undefined;
	for await (const message of messages) {
		if (message.type === "result") {
			result = message;
		}
	}
	if (!result) {
		throw new Error("Claude Code finished without a result.");
	}
	return result;
}

function abortControllerFollowing(signal: AbortSignal | undefined): AbortController | undefined {
	if (!signal) {
		return undefined;
	}

	const controller = new AbortController();
	if (signal.aborted) {
		controller.abort(signal.reason);
	} else {
		signal.addEventListener("abort", () => controller.abort(signal.reason), { once: true });
	}
	return controller;
}

function environmentWithoutParentSession(): NodeJS.ProcessEnv {
	const environment = { ...process.env };
	for (const name of PARENT_SESSION_VARIABLES) {
		delete environment[name];
	}
	return environment;
}
