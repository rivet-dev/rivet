import { randomUUID } from "node:crypto";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fauxAssistantMessage } from "@earendil-works/pi-ai";
import { setup } from "rivetkit";
import { setupTest } from "rivetkit/test";
import { afterAll, beforeAll, describe, expect, test, vi } from "vitest";
import { pi } from "../src/index.js";
import { createMockModel, type MockModel } from "./helpers/mock-model.js";

let mockModel: MockModel;
let hostAgentDir: string;
let previousAgentDir: string | undefined;
let registry: ReturnType<typeof buildRegistry>;

function buildRegistry(mock: MockModel) {
	const providers = { mock: mock.providerConfig };
	const napActions = {
		nap: (c: { sleep: () => void }) => {
			c.sleep();
		},
	};
	const agent = pi({
		providers,
		apiKeys: { mock: "key-from-config" },
		model: "mock/mock-model",
		scopedModels: ["mock/mock-model", "mock/mock-model-2"],
		state: { sleeps: 0 },
		onSleep: (c) => {
			c.state.sleeps += 1;
		},
		actions: { ...napActions, sleeps: (c) => c.state.sleeps },
	});
	const noKey = pi({
		providers,
		model: "mock/mock-model",
	});
	return setup({ use: { agent, noKey } });
}

beforeAll(async () => {
	mockModel = createMockModel();
	mockModel.reply("say hello", fauxAssistantMessage("Hi there!"));

	hostAgentDir = await mkdtemp(join(tmpdir(), "rivet-pi-host-agent-"));
	await writeFile(
		join(hostAgentDir, "auth.json"),
		JSON.stringify({ mock: { type: "api_key", key: "key-from-host-file" } }),
	);
	previousAgentDir = process.env.PI_CODING_AGENT_DIR;
	process.env.PI_CODING_AGENT_DIR = hostAgentDir;

	registry = buildRegistry(mockModel);
});

afterAll(async () => {
	if (previousAgentDir === undefined) delete process.env.PI_CODING_AGENT_DIR;
	else process.env.PI_CODING_AGENT_DIR = previousAgentDir;
	if (hostAgentDir) await rm(hostAgentDir, { recursive: true, force: true });
});

describe("pi model selection", () => {
	test("a client cannot switch to a model outside the allowlist", async (c) => {
		const { client } = await setupTest(c, registry);
		const handle = client.agent.getOrCreate(["not-allowed", randomUUID()]);

		expect((await handle.getAvailableModels()).map((model) => model.id)).toEqual([
			"mock-model",
			"mock-model-2",
		]);
		await expect(handle.setModel("mock", "mock-model-3")).rejects.toMatchObject({
			group: "user",
			code: "model_not_allowed",
		});
		expect((await handle.getSession()).model?.id).toBe("mock-model");
	});

	test("the model a client switched to is still used after the actor sleeps", async (c) => {
		const { client } = await setupTest(c, registry);
		const handle = client.agent.getOrCreate(["switch-sleep", randomUUID()]);
		await handle.setModel("mock", "mock-model-2");

		await handle.nap();
		// Sleep finishes after the `nap` action returns, so poll the persisted
		// counter until the sleep hook has run.
		await vi.waitFor(async () => {
			expect(await handle.sleeps()).toBeGreaterThanOrEqual(1);
		});

		const before = mockModel.requests.length;
		await handle.prompt("say hello");
		expect(mockModel.requests.slice(before).map((request) => request.model)).toEqual(["mock-model-2"]);
	});

	test("requests use the configured key, never a Pi login file on the server", async (c) => {
		const { client } = await setupTest(c, registry);
		const handle = client.agent.getOrCreate(["config-key", randomUUID()]);
		const before = mockModel.requests.length;
		await handle.prompt("say hello");
		expect(mockModel.requests.slice(before).map((request) => request.apiKey)).toEqual(["key-from-config"]);

		const noKey = client.noKey.getOrCreate(["host-file", randomUUID()]);
		expect(await noKey.getAvailableModels()).toEqual([]);
		await expect(noKey.setModel("mock", "mock-model")).rejects.toMatchObject({
			code: "model_unavailable",
		});
	});
});
