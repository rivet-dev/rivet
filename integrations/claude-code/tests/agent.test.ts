import { randomUUID } from "node:crypto";
import { setup } from "rivetkit";
import { setupTest } from "rivetkit/test";
import { describe, expect, test } from "vitest";
import { claudeCodeAgent } from "../src/agent.ts";
import { FAKE_CLAUDE_CODE, optionValue, parseEcho, withFakeClaude } from "./fake-claude.ts";

const researcher = claudeCodeAgent({
	systemPrompt: "Research.",
	tools: ["WebSearch"],
	claudeCodePath: FAKE_CLAUDE_CODE,
});

const registry = setup({ use: { researcher } });

describe("claudeCodeAgent", () => {
	test("continues the same Claude Code session across prompts", async (t) => {
		const { client } = await setupTest(t, registry);
		const agent = client.researcher.getOrCreate([randomUUID()]);

		const first = await agent.prompt("First question");
		const second = await agent.prompt("Follow-up question");

		const secondEcho = parseEcho(second.response);
		expect(optionValue(secondEcho.args, "--resume")).toBe(first.sessionId);
		expect(second.sessionId).toBe(first.sessionId);
		expect(await agent.getTurns()).toMatchObject([{ prompt: "First question" }, { prompt: "Follow-up question" }]);
	});

	test("rejects a prompt while a turn is in progress", async (t) => {
		const { client } = await setupTest(t, registry);
		const connection = client.researcher.getOrCreate([randomUUID()]).connect();

		await withFakeClaude({ delayMs: 2_000 }, async () => {
			const firstTurn = connection.prompt("Long task");
			await expect(connection.prompt("Interrupting task")).rejects.toMatchObject({
				code: "claude_code_turn_in_progress",
			});
			await expect(firstTurn).resolves.toMatchObject({ prompt: "Long task" });
		});
		await connection.dispose();
	});
});
