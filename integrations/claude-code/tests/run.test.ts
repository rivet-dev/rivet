import { realpathSync } from "node:fs";
import { tmpdir } from "node:os";
import { describe, expect, test } from "vitest";
import { z } from "zod";
import { type ClaudeCodeRequest, runClaudeCode } from "../src/run.ts";
import { FAKE_CLAUDE_CODE, optionValue, parseEcho, withFakeClaude } from "./fake-claude.ts";

const planSchema = z.object({ subQuestions: z.array(z.string()) });

async function runAndEcho(request: Partial<ClaudeCodeRequest> = {}) {
	const run = await runClaudeCode({
		prompt: "Hello",
		systemPrompt: "Be brief.",
		claudeCodePath: FAKE_CLAUDE_CODE,
		...request,
	});
	return { run, echo: parseEcho(run.output) };
}

describe("runClaudeCode", () => {
	test("sends the prompt and system prompt and returns the text result", async () => {
		const { run, echo } = await runAndEcho({ prompt: "What is Rivet?" });

		expect(echo.prompt).toBe("What is Rivet?");
		expect(echo.systemPrompt).toEqual(["Be brief."]);
		expect(run).toMatchObject({ costUsd: 0.01, durationMs: 5 });
	});

	test("enables and auto-approves only the requested tools and denies the rest", async () => {
		const { echo } = await runAndEcho({ tools: ["WebSearch", "WebFetch"] });

		expect(optionValue(echo.args, "--tools")?.split(",")).toEqual(["WebSearch", "WebFetch"]);
		expect(optionValue(echo.args, "--allowedTools")?.split(",")).toEqual(["WebSearch", "WebFetch"]);
		expect(optionValue(echo.args, "--permission-mode")).toBe("dontAsk");
	});

	test("disables all tools when none are requested", async () => {
		const { echo } = await runAndEcho();

		expect(optionValue(echo.args, "--tools")).toBe("");
	});

	test("ignores the user's Claude Code settings and CLAUDE.md files", async () => {
		const { echo } = await runAndEcho();

		expect(optionValue(echo.args, "--setting-sources")).toBe("");
	});

	test("does not persist the session unless asked to", async () => {
		const { echo } = await runAndEcho();

		expect(echo.args).toContain("--no-session-persistence");
	});

	test("persists and resumes a session when asked to", async () => {
		const { run, echo } = await runAndEcho({ persistSession: true, resumeSessionId: "session-1" });

		expect(echo.args).not.toContain("--no-session-persistence");
		expect(run.sessionId).toBe("session-1");
	});

	test("runs Claude Code in the requested working directory", async () => {
		const workingDirectory = realpathSync(tmpdir());
		const { echo } = await runAndEcho({ cwd: workingDirectory });

		expect(realpathSync(echo.cwd)).toBe(workingDirectory);
	});

	test("does not pass the parent Claude Code session to the run", async () => {
		process.env.CLAUDECODE = "1";
		try {
			const { echo } = await runAndEcho();
			expect(echo.parentSession).toBeNull();
		} finally {
			delete process.env.CLAUDECODE;
		}
	});

	test("returns structured output validated against the schema", async () => {
		const run = await withFakeClaude({ structuredOutput: { subQuestions: ["a", "b"] } }, () =>
			runClaudeCode({ prompt: "Plan", systemPrompt: "Plan.", claudeCodePath: FAKE_CLAUDE_CODE }, planSchema),
		);

		expect(run.output).toEqual({ subQuestions: ["a", "b"] });
	});

	test("sends the schema as JSON Schema draft-07, the dialect Claude Code validates", async () => {
		const echoedSchema = z.object({ jsonSchema: z.record(z.string(), z.unknown()) });
		const run = await withFakeClaude({ echoAsStructuredOutput: true }, () =>
			runClaudeCode({ prompt: "Plan", systemPrompt: "Plan.", claudeCodePath: FAKE_CLAUDE_CODE }, echoedSchema),
		);

		expect(run.output.jsonSchema.$schema).toBe("http://json-schema.org/draft-07/schema#");
	});

	test("rejects structured output that does not match the schema", async () => {
		await expect(
			withFakeClaude({ structuredOutput: { subQuestions: "oops" } }, () =>
				runClaudeCode({ prompt: "Plan", systemPrompt: "Plan.", claudeCodePath: FAKE_CLAUDE_CODE }, planSchema),
			),
		).rejects.toThrow();
	});

	test("throws with the reason when Claude Code reports a failed run", async () => {
		await expect(withFakeClaude({ failWith: "error_during_execution" }, () => runAndEcho())).rejects.toThrow(
			"Claude Code run failed (error_during_execution): fake failure",
		);
	});

	test("stops the run when the signal aborts", async () => {
		const startedAt = Date.now();

		await expect(
			withFakeClaude({ delayMs: 10_000 }, () => runAndEcho({ signal: AbortSignal.timeout(200) })),
		).rejects.toThrow("Operation aborted");
		expect(Date.now() - startedAt).toBeLessThan(5_000);
	});
});
