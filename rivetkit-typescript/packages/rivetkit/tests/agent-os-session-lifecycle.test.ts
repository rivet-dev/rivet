import { LLMock } from "@copilotkit/llmock";
import { afterAll, beforeAll, describe, expect, test } from "vitest";
import { agentOs } from "@/agent-os/index";
import { setup } from "@/mod";
import { setupTest } from "@/test/mod";
import common from "@rivet-dev/agent-os-common";
import pi from "@rivet-dev/agent-os-pi";

describe("agentOS session lifecycle", () => {
	let mock: LLMock;
	let mockUrl: string;
	let mockPort: number;

	beforeAll(async () => {
		mock = new LLMock({ port: 0, logLevel: "silent" });
		mock.addFixtures([
			{ match: { predicate: () => true }, response: { content: "Hello from mock LLM" } },
		]);
		mockUrl = await mock.start();
		mockPort = Number(new URL(mockUrl).port);
	});

	afterAll(async () => {
		await mock.stop();
	});

	function createRegistry() {
		const vm = agentOs({
			options: {
				software: [common, pi],
				loopbackExemptPorts: [mockPort],
			},
		});
		return setup({ use: { vm } });
	}

	test("writeFile, readFile, exec", async (c) => {
		const { client } = await setupTest(c, createRegistry());
		const actor = (client as any).vm.getOrCreate([`basic-${crypto.randomUUID()}`]);

		await actor.writeFile("/tmp/test.txt", "hello");
		const data = await actor.readFile("/tmp/test.txt");
		expect(new TextDecoder().decode(data)).toBe("hello");

		const result = await actor.exec("echo works");
		expect(result.exitCode).toBe(0);
		expect(result.stdout.trim()).toBe("works");
	}, 60_000);

	test("create session, send prompt, close session", async (c) => {
		const { client } = await setupTest(c, createRegistry());
		const actor = (client as any).vm.getOrCreate([`session-${crypto.randomUUID()}`]);

		const session = await actor.createSession("pi", {
			env: {
				ANTHROPIC_API_KEY: "mock-key",
				ANTHROPIC_BASE_URL: mockUrl,
			},
		});
		expect(session.sessionId).toBeTruthy();

		const response = await actor.sendPrompt(session.sessionId, "Say hello");
		expect(response).toBeTruthy();
		expect(response.response).toBeTruthy();
		expect(response.text).toBeTypeOf("string");

		await actor.closeSession(session.sessionId);
	}, 120_000);
});
