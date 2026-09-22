import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { LLMock } from "@copilotkit/aimock";
import { ModelRuntime } from "@earendil-works/pi-coding-agent";

export type PiModel = NonNullable<ReturnType<ModelRuntime["getModel"]>>;

export interface MockModel {
	mock: LLMock;
	model: PiModel;
	modelRuntime: ModelRuntime;
	dispose(): Promise<void>;
}

/**
 * Starts an OpenAI-compatible mock LLM server and a Pi `ModelRuntime` that
 * knows it as provider `mock`. Credentials live in a temp dir so the test
 * never reads or writes `~/.pi`.
 */
export async function startMockModel(): Promise<MockModel> {
	const mock = new LLMock({ port: 0 });
	await mock.start();
	const agentDir = await mkdtemp(join(tmpdir(), "rivet-pi-test-agent-"));
	const modelRuntime = await ModelRuntime.create({
		authPath: join(agentDir, "auth.json"),
		modelsPath: null,
	});
	modelRuntime.registerProvider("mock", {
		baseUrl: `${mock.url}/v1`,
		api: "openai-completions",
		apiKey: "mock",
		models: [
			{
				id: "mock-model",
				name: "Mock model",
				reasoning: false,
				input: ["text"],
				cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
				contextWindow: 128_000,
				maxTokens: 4_096,
			},
		],
	});
	await modelRuntime.refresh({ allowNetwork: false });
	const model = modelRuntime.getModel("mock", "mock-model");
	if (!model) {
		throw new Error("mock model was not registered");
	}
	return {
		mock,
		model,
		modelRuntime,
		dispose: async () => {
			await mock.stop();
			await rm(agentDir, { recursive: true, force: true });
		},
	};
}
