import { setupTest } from "rivetkit/test";
import { expect, test, vi } from "vitest";
import { registry } from "../src/actors.ts";

// Mock the AI SDK and OpenAI
vi.mock("@ai-sdk/openai", () => ({
	openai: () => "mock-model",
}));

vi.mock("ai", () => ({
	generateText: vi.fn().mockImplementation(async ({ prompt }) => ({
		text: `AI response to: ${prompt}`,
	})),
	tool: vi.fn().mockImplementation(({ execute }) => ({ execute })),
}));

vi.mock("../src/my-tools.ts", () => ({
	getWeather: vi.fn().mockResolvedValue({
		location: "San Francisco",
		temperature: 72,
		condition: "sunny",
	}),
}));

test("AI Agent can handle basic actions without connection", async (ctx) => {
	const { client } = await setupTest(ctx, registry);
	const agent = client.aiAgent.getOrCreate(["test-basic"]);

	// Test initial state
	const initialMessages = await agent.getMessages();
	expect(initialMessages).toEqual([]);

	// Send a message
	const userMessage = "Hello, how are you?";
	const response = await agent.sendMessage(userMessage);

	// Verify response structure
	expect(response).toMatchObject({
		role: "assistant",
		content: expect.stringContaining("AI response to: Hello, how are you?"),
		timestamp: expect.any(Number),
	});

	// Verify messages are stored
	const messages = await agent.getMessages();
	expect(messages).toHaveLength(2);
	expect(messages[0]).toMatchObject({
		role: "user",
		content: userMessage,
		timestamp: expect.any(Number),
	});
	expect(messages[1]).toEqual(response);
});

test("AI Agent maintains conversation history", async (ctx) => {
	const { client } = await setupTest(ctx, registry);
	const agent = client.aiAgent.getOrCreate(["test-history"]);

	// Send multiple messages
	await agent.sendMessage("First message");
	await agent.sendMessage("Second message");
	await agent.sendMessage("Third message");

	const messages = await agent.getMessages();
	expect(messages).toHaveLength(6); // 3 user + 3 assistant messages

	// Verify message ordering and roles
	expect(messages[0].role).toBe("user");
	expect(messages[0].content).toBe("First message");
	expect(messages[1].role).toBe("assistant");
	expect(messages[2].role).toBe("user");
	expect(messages[2].content).toBe("Second message");
	expect(messages[3].role).toBe("assistant");
	expect(messages[4].role).toBe("user");
	expect(messages[4].content).toBe("Third message");
	expect(messages[5].role).toBe("assistant");
});

test("AI Agent handles weather tool usage", async (ctx) => {
	const { client } = await setupTest(ctx, registry);
	const agent = client.aiAgent.getOrCreate(["test-weather"]);

	// Send a weather-related message
	const response = await agent.sendMessage(
		"What's the weather in San Francisco?",
	);

	// Verify response was generated
	expect(response.role).toBe("assistant");
	expect(response.content).toContain(
		"AI response to: What's the weather in San Francisco?",
	);
	expect(response.timestamp).toBeGreaterThan(0);

	// Verify message history includes both user and assistant messages
	const messages = await agent.getMessages();
	expect(messages).toHaveLength(2);
	expect(messages[0].content).toBe("What's the weather in San Francisco?");
	expect(messages[1]).toEqual(response);
});

test("AI Agent timestamps are sequential", async (ctx) => {
	const { client } = await setupTest(ctx, registry);
	const agent = client.aiAgent.getOrCreate(["test-timestamps"]);

	const response1 = await agent.sendMessage("First");
	const response2 = await agent.sendMessage("Second");

	expect(response2.timestamp).toBeGreaterThanOrEqual(response1.timestamp);

	const messages = await agent.getMessages();
	for (let i = 1; i < messages.length; i++) {
		expect(messages[i].timestamp).toBeGreaterThanOrEqual(
			messages[i - 1].timestamp,
		);
	}
});
