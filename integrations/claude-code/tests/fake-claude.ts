import { fileURLToPath } from "node:url";
import { z } from "zod";

export const FAKE_CLAUDE_CODE = fileURLToPath(new URL("./fake-claude-process.ts", import.meta.url));

const fakeClaudeEchoSchema = z.object({
	prompt: z.string(),
	systemPrompt: z.array(z.string()).nullable(),
	jsonSchema: z.record(z.string(), z.unknown()).nullable(),
	args: z.array(z.string()),
	cwd: z.string(),
	parentSession: z.string().nullable(),
});

export type FakeClaudeEcho = z.infer<typeof fakeClaudeEchoSchema>;

export function parseEcho(response: string): FakeClaudeEcho {
	return fakeClaudeEchoSchema.parse(JSON.parse(response));
}

export interface FakeClaudeBehavior {
	failWith?: "error_during_execution";
	structuredOutput?: unknown;
	echoAsStructuredOutput?: boolean;
	delayMs?: number;
}

export async function withFakeClaude<T>(behavior: FakeClaudeBehavior, work: () => Promise<T>): Promise<T> {
	const variables: Record<string, string | undefined> = {
		FAKE_CLAUDE_FAIL_WITH: behavior.failWith,
		FAKE_CLAUDE_STRUCTURED_OUTPUT: behavior.structuredOutput === undefined ? undefined : JSON.stringify(behavior.structuredOutput),
		FAKE_CLAUDE_ECHO_AS_STRUCTURED_OUTPUT: behavior.echoAsStructuredOutput ? "1" : undefined,
		FAKE_CLAUDE_DELAY_MS: behavior.delayMs?.toString(),
	};
	for (const [name, value] of Object.entries(variables)) {
		if (value !== undefined) {
			process.env[name] = value;
		}
	}
	try {
		return await work();
	} finally {
		for (const name of Object.keys(variables)) {
			delete process.env[name];
		}
	}
}

export function optionValue(args: string[], option: string): string | undefined {
	return args.find((arg) => arg.startsWith(`${option}=`))?.slice(option.length + 1);
}
