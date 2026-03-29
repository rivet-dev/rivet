import type { Fixture, FixtureResponse, ToolCall } from "@copilotkit/llmock";
import { LLMock } from "@copilotkit/llmock";

/**
 * Default fixture that matches any message and returns a simple text response.
 */
export const DEFAULT_TEXT_FIXTURE: Fixture = {
	match: { predicate: () => true },
	response: { content: "Hello from llmock" },
};

/**
 * Create an Anthropic Messages API fixture for text content responses.
 */
export function createAnthropicFixture(
	match: {
		userMessage?: string | RegExp;
		predicate?: (req: unknown) => boolean;
	},
	response: { content?: string; toolCalls?: ToolCall[] },
): Fixture {
	const fixtureResponse: FixtureResponse = response.toolCalls
		? { toolCalls: response.toolCalls }
		: { content: response.content ?? "" };
	return { match, response: fixtureResponse };
}

/**
 * Start an LLMock server on a random port.
 * Returns the mock instance and its base URL.
 */
export async function startLlmock(
	fixtures?: Fixture[],
): Promise<{ url: string; mock: LLMock }> {
	const mock = new LLMock({ port: 0, logLevel: "silent" });
	if (fixtures) {
		mock.addFixtures(fixtures);
	}
	const url = await mock.start();
	return { url, mock };
}

/**
 * Stop a running LLMock server.
 */
export async function stopLlmock(mock: LLMock): Promise<void> {
	await mock.stop();
}
