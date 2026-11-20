import { ConfigProvider, Effect } from "effect";
import { LanguageModel } from "effect/ai";
import {
	afterAll,
	afterEach,
	beforeAll,
	describe,
	expect,
	it,
	vi,
} from "vitest";
import { OpenAiModelLayer } from "../src/model.ts";

const fetch = vi.fn(async (..._args: Parameters<typeof globalThis.fetch>) =>
	Response.json({
		id: "resp_fixture",
		object: "response",
		created_at: 0,
		model: "fixture-model",
		output: [
			{
				id: "msg_fixture",
				type: "message",
				status: "completed",
				role: "assistant",
				content: [
					{
						type: "output_text",
						text: "Fixture reply.",
						annotations: [],
					},
				],
			},
		],
	}),
);

beforeAll(() => {
	vi.stubGlobal("fetch", fetch);
});

afterEach(() => {
	fetch.mockClear();
});

afterAll(() => {
	vi.unstubAllGlobals();
});

describe("OpenAiModelLayer", () => {
	it.each([
		[undefined, "https://api.openai.com/v1", "gpt-6-luna"],
		[
			"https://api.openai.com/v1",
			"https://api.openai.com/v1",
			"gpt-6-luna",
		],
		[
			"https://gateway.example/v1",
			"https://gateway.example/v1",
			"gpt-4o-mini",
		],
	])("uses the right model for OPENAI_BASE_URL=%s", async (baseUrl, endpoint, model) => {
		const config = ConfigProvider.fromEnvRecord({
			OPENAI_API_KEY: "offline-test",
			OPENAI_BASE_URL: baseUrl,
		});

		const response = await Effect.runPromise(
			LanguageModel.generateText({ prompt: "Hello" }).pipe(
				Effect.provide(OpenAiModelLayer),
				Effect.provide(ConfigProvider.layer(config)),
			),
		);

		expect(response.text).toBe("Fixture reply.");
		expect(fetch).toHaveBeenCalledTimes(1);
		const request = new Request(...fetch.mock.calls[0]);
		expect(request.url).toBe(`${endpoint}/responses`);
		expect(request.method).toBe("POST");
		expect(await request.json()).toMatchObject({ model });
	});
});
