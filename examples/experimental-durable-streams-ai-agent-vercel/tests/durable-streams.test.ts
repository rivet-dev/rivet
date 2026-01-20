// import { DurableStreamTestServer } from "@durable-streams/server";
// import { setupTest } from "rivetkit/test";
// import { afterAll, beforeAll, expect, test, vi } from "vitest";
//
// // Start a real durable streams test server
// let streamsServer: DurableStreamTestServer;
//
// beforeAll(async () => {
// 	streamsServer = new DurableStreamTestServer({
// 		port: 0, // Use random available port
// 		host: "127.0.0.1",
// 		longPollTimeout: 5_000,
// 	});
// 	const url = await streamsServer.start();
// 	// Set the URL for the registry to use
// 	process.env.STREAMS_SERVER_URL = url;
// });
//
// afterAll(async () => {
// 	// Close all connections immediately - the actors have long-poll connections that would otherwise hang
// 	// @ts-expect-error - closeAllConnections() available in Node 18.2+
// 	streamsServer["server"]?.closeAllConnections?.();
// 	await streamsServer.stop();
// }, 5_000);
//
// // Only mock Anthropic AI SDK if the API key is not set
// if (!process.env.ANTHROPIC_API_KEY) {
// 	vi.mock("@ai-sdk/anthropic", () => ({
// 		anthropic: () => "mock-anthropic-model",
// 	}));
//
// 	vi.mock("ai", () => ({
// 		streamText: vi.fn().mockImplementation(({ prompt }) => ({
// 			textStream: (async function* () {
// 				yield `Response to: ${prompt}`;
// 			})(),
// 		})),
// 	}));
// }
//
// // Import registry after environment is set up
// const { registry } = await import("../src/actors.ts");
//
// test("AI Agent initializes with conversation ID", async (ctx) => {
// 	const { client } = await setupTest(ctx, registry);
//
// 	const conversationId = "test-conversation-123";
// 	const agent = client.aiAgent.getOrCreate([conversationId], {
// 		createWithInput: { conversationId },
// 	});
//
// 	const result = await agent.getConversationId();
// 	expect(result).toBe(conversationId);
// });
//
// test("AI Agent starts with empty processed prompts", async (ctx) => {
// 	const { client } = await setupTest(ctx, registry);
//
// 	const conversationId = "test-empty-prompts";
// 	const agent = client.aiAgent.getOrCreate([conversationId], {
// 		createWithInput: { conversationId },
// 	});
//
// 	const processed = await agent.getProcessedPrompts();
// 	expect(processed).toEqual([]);
// });
//
// test("AI Agent maintains separate state per conversation", async (ctx) => {
// 	const { client } = await setupTest(ctx, registry);
//
// 	const agent1 = client.aiAgent.getOrCreate(["conversation-1"], {
// 		createWithInput: { conversationId: "conversation-1" },
// 	});
//
// 	const agent2 = client.aiAgent.getOrCreate(["conversation-2"], {
// 		createWithInput: { conversationId: "conversation-2" },
// 	});
//
// 	const id1 = await agent1.getConversationId();
// 	const id2 = await agent2.getConversationId();
//
// 	expect(id1).toBe("conversation-1");
// 	expect(id2).toBe("conversation-2");
// 	expect(id1).not.toBe(id2);
// });
